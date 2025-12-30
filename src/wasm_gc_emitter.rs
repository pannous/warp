use crate::node::{Node, Grouper, Bracket, DataType};
use crate::extensions::numbers::Number;
use wasm_encoder::*;
use std::collections::HashMap;
use Instruction::I32Const;
use wasmparser::{Validator, WasmFeatures};
use log::trace;
use crate::wasm_gc_reader::read_bytes;
use crate::wasp_parser::WaspParser;

/// WebAssembly GC emitter for Node AST
/// Generates WASM GC bytecode using struct and array types
pub struct WasmGcEmitter {
    module: Module,
    types: TypeSection,
    functions: FunctionSection,
    code: CodeSection,
    exports: ExportSection,
    names: NameSection,
    memory: MemorySection,
    data: DataSection,
    // Type indices for unified GC types
    node_base_type: u32,
    node_array_type: u32,
    next_type_idx: u32,
    next_func_idx: u32,
    // String storage for linear memory
    string_table: HashMap<String, u32>, // Maps string -> memory offset
    next_data_offset: u32,
}

/// Node variant tags (for runtime type checking)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Empty = 0,
    Number = 1,
    Text = 2,
    Codepoint = 3,
    Symbol = 4,
    KeyValue = 5,
    Pair = 6,
    Tag = 7,
    Block = 8,
    List = 9,
    Data = 10,
    WithMeta = 11,
}

impl WasmGcEmitter {
    pub fn new() -> Self {
        WasmGcEmitter {
            module: Module::new(),
            types: TypeSection::new(),
            functions: FunctionSection::new(),
            code: CodeSection::new(),
            exports: ExportSection::new(),
            names: NameSection::new(),
            memory: MemorySection::new(),
            data: DataSection::new(),
            node_base_type: 0,
            node_array_type: 0,
            next_type_idx: 0,
            next_func_idx: 0,
            string_table: HashMap::new(),
            next_data_offset: 8, // Start at offset 8 to avoid confusion with null (0)
        }
    }

    /// Generate all type definitions and functions
    pub fn emit(&mut self) {
        // Initialize linear memory (1 page = 64KB)
        self.memory.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        // Export memory so it can be accessed from the host
        self.exports.export("memory", ExportKind::Memory, 0);

        self.emit_gc_types();
        self.emit_constructor_functions();
    }

    /// Allocate a string in linear memory and return its offset
    /// Returns (ptr, len) tuple
    fn allocate_string(&mut self, s: &str) -> (u32, u32) {
        if let Some(&offset) = self.string_table.get(s) {
            return (offset, s.len() as u32);
        }

        let offset = self.next_data_offset;
        let bytes = s.as_bytes();

        // Add string data to data section (passive data, offset 0, memory index 0)
        self.data.active(
            0, // memory index
            &ConstExpr::i32_const(offset as i32), // offset expression
            bytes.iter().copied(),
        );

        self.string_table.insert(s.to_string(), offset);
        self.next_data_offset += bytes.len() as u32;

        (offset, bytes.len() as u32)
    }

    /// Define GC struct types for Node variants
    fn emit_gc_types(&mut self) {
        // First, define the unified Node struct type per the design spec
        // This is a single struct that can represent any node type
        // (type $node (struct
        //   (field $name_ptr i32)
        //   (field $name_len i32)
        //   (field $tag i32)
        //   (field $int_value i64)
        //   (field $float_value f64)
        //   (field $text_ptr i32)
        //   (field $text_len i32)
        //   (field $left (ref null $node))  // recursive reference
        //   (field $right (ref null $node))
        //   (field $meta (ref null $node))
        // ))

        // Save the index where we'll define the node struct
        let node_type_idx = self.next_type_idx;
        self.next_type_idx += 1;

        // For recursive refs, use the type index that will be assigned
        let node_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(node_type_idx),
        };

        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // name_ptr
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // name_len
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // tag
            FieldType { element_type: StorageType::Val(ValType::I64), mutable: false }, // int_value
            FieldType { element_type: StorageType::Val(ValType::F64), mutable: false }, // float_value
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // text_ptr
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // text_len
            FieldType { element_type: StorageType::Val(ValType::Ref(node_ref)), mutable: false }, // left
            FieldType { element_type: StorageType::Val(ValType::Ref(node_ref)), mutable: false }, // right
            FieldType { element_type: StorageType::Val(ValType::Ref(node_ref)), mutable: false }, // meta
        ]);
        self.node_base_type = node_type_idx;

        // Define node array type: array of (ref null node)
        let node_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.node_base_type),
        };
        let storage_type = StorageType::Val(ValType::Ref(node_ref));
        self.types.ty().array(&storage_type, true);
        self.node_array_type = self.next_type_idx;
        self.next_type_idx += 1;
    }

    /// Emit constructor functions for creating Node instances using unified struct
    fn emit_constructor_functions(&mut self) {
        // make_empty() -> (ref node)
        let node_ref = RefType {
            nullable: false,
            heap_type: HeapType::Concrete(self.node_base_type),
        };
        let func_type = self.types.len();
        self.types.ty().function(vec![], vec![ValType::Ref(node_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![]);
        // Emit all 10 fields: name_ptr, name_len, tag, int_value, float_value, text_ptr, text_len, left, right, meta
        func.instruction(&I32Const(0)); // name_ptr
        func.instruction(&I32Const(0)); // name_len
        func.instruction(&I32Const(NodeKind::Empty as i32)); // tag
        func.instruction(&Instruction::I64Const(0)); // int_value
        func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits()))); // float_value
        func.instruction(&I32Const(0)); // text_ptr
        func.instruction(&I32Const(0)); // text_len
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // left
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // right
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // meta
        func.instruction(&Instruction::StructNew(self.node_base_type));
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("make_empty", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // make_int(i64) -> (ref node)
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::I64], vec![ValType::Ref(node_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![(1, ValType::I64)]);
        func.instruction(&I32Const(0)); // name_ptr
        func.instruction(&I32Const(0)); // name_len
        func.instruction(&I32Const(NodeKind::Number as i32)); // tag
        func.instruction(&Instruction::LocalGet(0)); // int_value
        func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits()))); // float_value (unused)
        func.instruction(&I32Const(0)); // text_ptr
        func.instruction(&I32Const(0)); // text_len
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // left
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // right
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // meta
        func.instruction(&Instruction::StructNew(self.node_base_type));
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("make_int", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // make_float(f64) -> (ref node)
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::F64], vec![ValType::Ref(node_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![(1, ValType::F64)]);
        func.instruction(&I32Const(0)); // name_ptr
        func.instruction(&I32Const(0)); // name_len
        func.instruction(&I32Const(NodeKind::Number as i32)); // tag
        func.instruction(&Instruction::I64Const(0)); // int_value (unused)
        func.instruction(&Instruction::LocalGet(0)); // float_value
        func.instruction(&I32Const(0)); // text_ptr
        func.instruction(&I32Const(0)); // text_len
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // left
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // right
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // meta
        func.instruction(&Instruction::StructNew(self.node_base_type));
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("make_float", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // make_codepoint(i32) -> (ref node)
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::I32], vec![ValType::Ref(node_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![(1, ValType::I32)]);
        func.instruction(&I32Const(0)); // name_ptr
        func.instruction(&I32Const(0)); // name_len
        func.instruction(&I32Const(NodeKind::Codepoint as i32)); // tag
        func.instruction(&Instruction::I64Const(0)); // int_value
        func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits()))); // float_value
        func.instruction(&Instruction::LocalGet(0)); // text_ptr (reusing for codepoint)
        func.instruction(&I32Const(0)); // text_len
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // left
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // right
        func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // meta
        func.instruction(&Instruction::StructNew(self.node_base_type));
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("make_codepoint", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // get_node_kind(ref node) -> i32 (returns tag field)
        let node_ref_nullable = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.node_base_type),
        };
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::Ref(node_ref_nullable)], vec![ValType::I32]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![(1, ValType::Ref(node_ref_nullable))]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::StructGet {
            struct_type_index: self.node_base_type,
            field_index: 2, // tag field is at index 2
        });
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("get_node_kind", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // Add field getters for the unified node struct
        self.emit_node_field_getters();
    }

    /// Emit getter functions for unified node struct fields
    fn emit_node_field_getters(&mut self) {
        let node_ref = RefType {
            nullable: false,
            heap_type: HeapType::Concrete(self.node_base_type),
        };

        // get_tag(node) -> i32
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::Ref(node_ref)], vec![ValType::I32]);
        self.functions.function(func_type);
        let mut func = Function::new(vec![(1, ValType::Ref(node_ref))]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::StructGet {
            struct_type_index: self.node_base_type,
            field_index: 2, // tag field
        });
        func.instruction(&Instruction::End);
        self.code.function(&func);
        self.exports.export("get_tag", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // get_int_value(node) -> i64
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::Ref(node_ref)], vec![ValType::I64]);
        self.functions.function(func_type);
        let mut func = Function::new(vec![(1, ValType::Ref(node_ref))]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::StructGet {
            struct_type_index: self.node_base_type,
            field_index: 3, // int_value field
        });
        func.instruction(&Instruction::End);
        self.code.function(&func);
        self.exports.export("get_int_value", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // get_float_value(node) -> f64
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::Ref(node_ref)], vec![ValType::F64]);
        self.functions.function(func_type);
        let mut func = Function::new(vec![(1, ValType::Ref(node_ref))]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::StructGet {
            struct_type_index: self.node_base_type,
            field_index: 4, // float_value field
        });
        func.instruction(&Instruction::End);
        self.code.function(&func);
        self.exports.export("get_float_value", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // get_name_len(node) -> i32
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::Ref(node_ref)], vec![ValType::I32]);
        self.functions.function(func_type);
        let mut func = Function::new(vec![(1, ValType::Ref(node_ref))]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::StructGet {
            struct_type_index: self.node_base_type,
            field_index: 1, // name_len field
        });
        func.instruction(&Instruction::End);
        self.code.function(&func);
        self.exports.export("get_name_len", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;
    }

    /// Emit a function that constructs and returns a specific Node
    pub fn emit_node_main(&mut self, node: &Node) {
        // Pre-allocate all strings in the node tree
        self.collect_and_allocate_strings(node);

        // Use the unified Node struct type
        let node_ref = RefType {
            nullable: false,
            heap_type: HeapType::Concrete(self.node_base_type),
        };

        let func_type = self.types.len();
        self.types.ty().function(vec![], vec![ValType::Ref(node_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![]);

        // Encode the node into the unified struct format
        self.emit_node_instructions(&mut func, node);

        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("main", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;
    }

    /// Recursively collect and allocate all strings from a node tree
    fn collect_and_allocate_strings(&mut self, node: &Node) {
        let node = node.unwrap_meta();
        match node {
            Node::Text(s) | Node::Symbol(s) => {
                self.allocate_string(s);
            }
            Node::Tag { title, params, body } => {
                self.allocate_string(title);
                self.collect_and_allocate_strings(params);
                self.collect_and_allocate_strings(body);
            }
            Node::KeyValue(key, value) => {
                self.allocate_string(key);
                self.collect_and_allocate_strings(value);
            }
            Node::Pair(left, right) => {
                self.collect_and_allocate_strings(left);
                self.collect_and_allocate_strings(right);
            }
            Node::Block(items, _, _) | Node::List(items) => {
                for item in items {
                    self.collect_and_allocate_strings(item);
                }
            }
            Node::Data(dada) => {
                self.allocate_string(&dada.type_name);
            }
            _ => {}
        }
    }

    /// Emit WASM instructions to construct a Node in the unified struct format
    fn emit_node_instructions(&self, func: &mut Function, node: &Node) {
        // Unwrap metadata if present
        let node = node.unwrap_meta();

        match node {
            Node::Empty => {
                // name_ptr, name_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // tag
                func.instruction(&I32Const(NodeKind::Empty as i32));
                // int_value, float_value
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr, text_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // left, right, meta (all null)
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Number(num) => {
                // name_ptr, name_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // tag
                func.instruction(&I32Const(NodeKind::Number as i32));
                // int_value, float_value
                match num {
                    Number::Int(i) => {
                        func.instruction(&Instruction::I64Const(*i));
                        func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                    }
                    Number::Float(f) => {
                        func.instruction(&Instruction::I64Const(0));
                        func.instruction(&Instruction::F64Const(Ieee64::new(f.to_bits())));
                    }
                    _ => {
                        // Quotient, Complex not yet supported
                        func.instruction(&Instruction::I64Const(0));
                        func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                    }
                }
                // text_ptr, text_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // left, right, meta (all null)
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Text(s) => {
                // name_ptr, name_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // tag
                func.instruction(&I32Const(NodeKind::Text as i32));
                // int_value, float_value
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr, text_len (use actual allocated string)
                let (ptr, len) = self.string_table.get(s.as_str())
                    .map(|&offset| (offset, s.len() as u32))
                    .unwrap_or((0, s.len() as u32));
                func.instruction(&I32Const(ptr as i32));
                func.instruction(&I32Const(len as i32));
                // left, right, meta
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Codepoint(c) => {
                // name_ptr, name_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // tag
                func.instruction(&I32Const(NodeKind::Codepoint as i32));
                // int_value (store codepoint as int), float_value
                func.instruction(&Instruction::I64Const(*c as i64));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr, text_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // left, right, meta
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Symbol(s) => {
                // name_ptr, name_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // tag
                func.instruction(&I32Const(NodeKind::Symbol as i32));
                // int_value, float_value
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr, text_len (use actual allocated string)
                let (ptr, len) = self.string_table.get(s.as_str())
                    .map(|&offset| (offset, s.len() as u32))
                    .unwrap_or((0, s.len() as u32));
                func.instruction(&I32Const(ptr as i32));
                func.instruction(&I32Const(len as i32));
                // left, right, meta
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Tag { title, params: _params, body: _body } => {
                // For Tag nodes, store the tag name in name field (use actual allocated string)
                let (ptr, len) = self.string_table.get(title.as_str())
                    .map(|&offset| (offset, title.len() as u32))
                    .unwrap_or((0, title.len() as u32));
                func.instruction(&I32Const(ptr as i32));
                func.instruction(&I32Const(len as i32));
                // tag
                func.instruction(&I32Const(NodeKind::Tag as i32));
                // int_value, float_value
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr, text_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // left (attrs), right (body), meta
                // Note: Recursive encoding not yet implemented - would need locals and multiple StructNew
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::KeyValue(key, value) => {
                // name_ptr, name_len (store key - use actual allocated string)
                let (ptr, len) = self.string_table.get(key.as_str())
                    .map(|&offset| (offset, key.len() as u32))
                    .unwrap_or((0, key.len() as u32));
                func.instruction(&I32Const(ptr as i32));
                func.instruction(&I32Const(len as i32));
                // tag
                func.instruction(&I32Const(NodeKind::KeyValue as i32));
                // int_value, float_value
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr, text_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // left (null), right (value node - not yet recursively encoded), meta
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                Self::emit_node_instructions(self,func,value); // todo : get Via local or Call!
                // func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // TODO: encode value
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Pair(_left, _right) => {
                // name_ptr, name_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // tag
                func.instruction(&I32Const(NodeKind::Pair as i32));
                // int_value, float_value
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr, text_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // left, right (not yet recursively encoded), meta
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // TODO: encode left
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // TODO: encode right
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Block(items, grouper, bracket) => {
                // name_ptr, name_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // tag
                func.instruction(&I32Const(NodeKind::Block as i32));
                // int_value (store grouper/bracket info as encoded value), float_value
                let grouper_val = match grouper {
                    Grouper::Object => 0i64,
                    Grouper::Group => 1,
                    Grouper::Pattern => 2,
                    Grouper::Expression => 3,
                    Grouper::Other(_, _) => 4,
                };
                let bracket_val = match bracket {
                    Bracket::Curly => 0i64,
                    Bracket::Square => 1,
                    Bracket::Round => 2,
                    Bracket::Other(_, _) => 3,
                };
                let group_info = grouper_val << 8 | bracket_val;
                func.instruction(&Instruction::I64Const(group_info));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr (item count), text_len
                func.instruction(&I32Const(items.len() as i32));
                func.instruction(&I32Const(0));
                // left (first item if exists), right, meta
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // TODO: encode items
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::List(items) => {
                // name_ptr, name_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // tag
                func.instruction(&I32Const(NodeKind::List as i32));
                // int_value, float_value
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr (item count), text_len
                func.instruction(&I32Const(items.len() as i32));
                func.instruction(&I32Const(0));
                // left (first item if exists), right, meta
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type))); // TODO: encode items
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Data(dada) => {
                // name_ptr, name_len (store type_name - use actual allocated string)
                let (ptr, len) = self.string_table.get(dada.type_name.as_str())
                    .map(|&offset| (offset, dada.type_name.len() as u32))
                    .unwrap_or((0, dada.type_name.len() as u32));
                func.instruction(&I32Const(ptr as i32));
                func.instruction(&I32Const(len as i32));
                // tag
                func.instruction(&I32Const(NodeKind::Data as i32));
                // int_value (store data_type), float_value
                let data_type_val = match &dada.data_type {
                    DataType::Vec => 0i64,
                    DataType::Tuple => 1,
                    DataType::Struct => 2,
                    DataType::Primitive => 3,
                    DataType::String => 4,
                    DataType::Other => 5,
                };
                func.instruction(&Instruction::I64Const(data_type_val));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                // text_ptr, text_len
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                // left, right, meta
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::WithMeta(_, _) => {
                // Should not reach here since unwrap_meta is called at the start
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(NodeKind::Empty as i32));
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
            Node::Error(_) => {
                // Emit an Empty node for errors
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(NodeKind::Empty as i32));
                func.instruction(&Instruction::I64Const(0));
                func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits())));
                func.instruction(&I32Const(0));
                func.instruction(&I32Const(0));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::RefNull(HeapType::Concrete(self.node_base_type)));
                func.instruction(&Instruction::StructNew(self.node_base_type));
            }
        }
    }



    fn validate_wasm(bytes: &Vec<u8>) {
        let mut features = WasmFeatures::default();
        features.set(WasmFeatures::REFERENCE_TYPES, true);
        features.set(WasmFeatures::GC, true);
        let mut validator = Validator::new_with_features(features);
        match validator.validate_all(&bytes) {
            Ok(_) => trace!("✓ WASM validation with GC features passed"),
            Err(e) => panic!("WASM validation failed: {}", e),
        }
    }


    /// Generate the final WASM module bytes
    pub fn finish(mut self) -> Vec<u8> {
        self.module.section(&self.types);
        self.module.section(&self.functions);
        self.module.section(&self.memory); // Memory section before code
        self.module.section(&self.exports);
        self.module.section(&self.code);
        self.module.section(&self.data); // Data section after code

        // Add comprehensive names for debugging
        self.emit_names();
        self.module.section(&self.names);

        let bytes = self.module.finish();

        // Save to file
        std::fs::write("test.wasm", &bytes).expect("Failed to write test.wasm");
        Self::validate_wasm(&bytes);
        bytes
    }

    /// Emit comprehensive WASM names for types, functions, and fields
    fn emit_names(&mut self) {
        // Module name
        self.names.module("wasp_node_ast");

        // Type names
        let mut type_names = NameMap::new();
        type_names.append(self.node_base_type, "node");
        type_names.append(self.node_array_type, "node_array");
        // Only unified node and node_array types
        self.names.types(&type_names);

        // Field names for the unified node struct
        let mut field_names = NameMap::new();
        field_names.append(0, "name_ptr");
        field_names.append(1, "name_len");
        field_names.append(2, "tag");
        field_names.append(3, "int_value");
        field_names.append(4, "float_value");
        field_names.append(5, "text_ptr");
        field_names.append(6, "text_len");
        field_names.append(7, "left");
        field_names.append(8, "right");
        field_names.append(9, "meta");

        let mut type_field_names = IndirectNameMap::new();
        type_field_names.append(self.node_base_type, &field_names);
        self.names.fields(&type_field_names);

        // Function names
        let mut func_names = NameMap::new();
        func_names.append(0, "make_empty");
        func_names.append(1, "make_int");
        func_names.append(2, "make_float");
        func_names.append(3, "make_codepoint");
        func_names.append(4, "get_node_kind");
        func_names.append(5, "get_tag");
        func_names.append(6, "get_int_value");
        func_names.append(7, "get_float_value");
        func_names.append(8, "get_name_len");
        func_names.append(9, "main");
        self.names.functions(&func_names);
    }
}


pub fn eval(code: &str) -> Node {
    let node = WaspParser::parse(code);
    if let Node::Error(e) = &node {
        panic!("Parse error: {}", e);
    }
    let mut emitter = WasmGcEmitter::new();
    emitter.emit();
    emitter.emit_node_main(&node);
    let wasm_bytes = emitter.finish();
    let obj = read_bytes(&wasm_bytes);
    match obj {
        Err(e) => panic!("Failed to read WASM bytes: {}", e),
        Ok(x) => Node::from_gc_object(&x)
    }
}
