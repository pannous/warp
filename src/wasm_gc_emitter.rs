use crate::node::{Node, Grouper, Bracket, DataType};
use crate::extensions::numbers::Number;
use wasm_encoder::*;

/// WebAssembly GC emitter for Node AST
/// Generates WASM GC bytecode using struct and array types
pub struct WasmGcEmitter {
    module: Module,
    types: TypeSection,
    functions: FunctionSection,
    code: CodeSection,
    exports: ExportSection,
    // Type indices for GC types
    node_base_type: u32,
    empty_type: u32,
    number_type: u32,
    text_type: u32,
    codepoint_type: u32,
    symbol_type: u32,
    keyvalue_type: u32,
    pair_type: u32,
    tag_type: u32,
    block_type: u32,
    list_type: u32,
    data_type: u32,
    meta_type: u32,
    withmeta_type: u32,
    node_array_type: u32,
    next_type_idx: u32,
    next_func_idx: u32,
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
            node_base_type: 0,
            empty_type: 0,
            number_type: 0,
            text_type: 0,
            codepoint_type: 0,
            symbol_type: 0,
            keyvalue_type: 0,
            pair_type: 0,
            tag_type: 0,
            block_type: 0,
            list_type: 0,
            data_type: 0,
            meta_type: 0,
            withmeta_type: 0,
            node_array_type: 0,
            next_type_idx: 0,
            next_func_idx: 0,
        }
    }

    /// Generate all type definitions and functions
    pub fn emit(&mut self) {
        self.emit_gc_types();
        self.emit_constructor_functions();
    }

    /// Define GC struct types for Node variants
    fn emit_gc_types(&mut self) {
        // Define node array type: array of (ref null node)
        let node_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.next_type_idx),
        };
        let storage_type = StorageType::Val(ValType::Ref(node_ref));
        self.types.ty().array(&storage_type, true);
        self.node_array_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Base Node type - abstract supertype (empty struct with tag)
        // Fields: [kind: i8]
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind tag
        ]);
        self.node_base_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Empty node: [kind: i8]
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
        ]);
        self.empty_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Number node: [kind: i8, is_int: i8, int_val: i64, float_val: f64]
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::I8, mutable: false }, // is_int flag
            FieldType { element_type: StorageType::Val(ValType::I64), mutable: false }, // int value
            FieldType { element_type: StorageType::Val(ValType::F64), mutable: false }, // float value
        ]);
        self.number_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Text node: [kind: i8, ptr: i32, len: i32]
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // ptr
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // len
        ]);
        self.text_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Codepoint node: [kind: i8, codepoint: i32]
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // codepoint
        ]);
        self.codepoint_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Symbol node: [kind: i8, ptr: i32, len: i32]
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // ptr
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // len
        ]);
        self.symbol_type = self.next_type_idx;
        self.next_type_idx += 1;

        // KeyValue node: [kind: i8, key_ptr: i32, key_len: i32, value: ref node]
        let value_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.node_base_type),
        };
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // key ptr
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // key len
            FieldType { element_type: StorageType::Val(ValType::Ref(value_ref)), mutable: false }, // value
        ]);
        self.keyvalue_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Pair node: [kind: i8, first: ref node, second: ref node]
        let node_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.node_base_type),
        };
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::Val(ValType::Ref(node_ref)), mutable: false }, // first
            FieldType { element_type: StorageType::Val(ValType::Ref(node_ref)), mutable: false }, // second
        ]);
        self.pair_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Tag node: [kind: i8, name_ptr: i32, name_len: i32, attrs: ref node, body: ref node]
        let node_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.node_base_type),
        };
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // name ptr
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // name len
            FieldType { element_type: StorageType::Val(ValType::Ref(node_ref)), mutable: false }, // attrs
            FieldType { element_type: StorageType::Val(ValType::Ref(node_ref)), mutable: false }, // body
        ]);
        self.tag_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Block/List node: [kind: i8, grouper: i8, bracket: i8, items: ref array]
        let array_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.node_array_type),
        };
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::I8, mutable: false }, // grouper
            FieldType { element_type: StorageType::I8, mutable: false }, // bracket
            FieldType { element_type: StorageType::Val(ValType::Ref(array_ref)), mutable: false }, // items
        ]);
        self.block_type = self.next_type_idx;
        self.list_type = self.next_type_idx; // List uses same structure
        self.next_type_idx += 1;

        // Data node: [kind: i8, type_name_ptr: i32, type_name_len: i32, data_type: i8]
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // type_name ptr
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // type_name len
            FieldType { element_type: StorageType::I8, mutable: false }, // data_type enum
        ]);
        self.data_type = self.next_type_idx;
        self.next_type_idx += 1;

        // Meta record: [comment_ptr: i32, comment_len: i32, line: i32, column: i32]
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // comment ptr
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // comment len
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // line
            FieldType { element_type: StorageType::Val(ValType::I32), mutable: false }, // column
        ]);
        self.meta_type = self.next_type_idx;
        self.next_type_idx += 1;

        // WithMeta node: [kind: i8, node: ref node, meta: ref meta]
        let node_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.node_base_type),
        };
        let meta_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.meta_type),
        };
        self.types.ty().struct_(vec![
            FieldType { element_type: StorageType::I8, mutable: false }, // kind
            FieldType { element_type: StorageType::Val(ValType::Ref(node_ref)), mutable: false }, // node
            FieldType { element_type: StorageType::Val(ValType::Ref(meta_ref)), mutable: false }, // meta
        ]);
        self.withmeta_type = self.next_type_idx;
        self.next_type_idx += 1;
    }

    /// Emit constructor functions for creating Node instances
    fn emit_constructor_functions(&mut self) {
        // make_empty() -> (ref empty)
        let empty_ref = RefType {
            nullable: false,
            heap_type: HeapType::Concrete(self.empty_type),
        };
        let func_type = self.types.len();
        self.types.ty().function(vec![], vec![ValType::Ref(empty_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::I32Const(NodeKind::Empty as i32));
        func.instruction(&Instruction::StructNew(self.empty_type));
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("make_empty", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // make_int(i64) -> (ref number)
        let number_ref = RefType {
            nullable: false,
            heap_type: HeapType::Concrete(self.number_type),
        };
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::I64], vec![ValType::Ref(number_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![(1, ValType::I64)]);
        func.instruction(&Instruction::I32Const(NodeKind::Number as i32));
        func.instruction(&Instruction::I32Const(1)); // is_int = true
        func.instruction(&Instruction::LocalGet(0)); // int value
        func.instruction(&Instruction::F64Const(Ieee64::new(0.0_f64.to_bits()))); // float value (unused)
        func.instruction(&Instruction::StructNew(self.number_type));
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("make_int", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // make_float(f64) -> (ref number)
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::F64], vec![ValType::Ref(number_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![(1, ValType::F64)]);
        func.instruction(&Instruction::I32Const(NodeKind::Number as i32));
        func.instruction(&Instruction::I32Const(0)); // is_int = false
        func.instruction(&Instruction::I64Const(0)); // int value (unused)
        func.instruction(&Instruction::LocalGet(0)); // float value
        func.instruction(&Instruction::StructNew(self.number_type));
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("make_float", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // make_codepoint(i32) -> (ref codepoint)
        let codepoint_ref = RefType {
            nullable: false,
            heap_type: HeapType::Concrete(self.codepoint_type),
        };
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::I32], vec![ValType::Ref(codepoint_ref)]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![(1, ValType::I32)]);
        func.instruction(&Instruction::I32Const(NodeKind::Codepoint as i32));
        func.instruction(&Instruction::LocalGet(0)); // codepoint value
        func.instruction(&Instruction::StructNew(self.codepoint_type));
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("make_codepoint", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // get_node_kind(ref node) -> i32
        let node_ref = RefType {
            nullable: true,
            heap_type: HeapType::Concrete(self.node_base_type),
        };
        let func_type = self.types.len();
        self.types.ty().function(vec![ValType::Ref(node_ref)], vec![ValType::I32]);
        self.functions.function(func_type);

        let mut func = Function::new(vec![(1, ValType::Ref(node_ref))]);
        func.instruction(&Instruction::LocalGet(0));
        func.instruction(&Instruction::StructGet {
            struct_type_index: self.node_base_type,
            field_index: 0,
        });
        func.instruction(&Instruction::End);

        self.code.function(&func);
        self.exports.export("get_node_kind", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;
    }

    /// Generate the final WASM module bytes
    pub fn finish(mut self) -> Vec<u8> {
        self.module.section(&self.types);
        self.module.section(&self.functions);
        self.module.section(&self.exports);
        self.module.section(&self.code);
        self.module.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_gc_types() {
        let mut emitter = WasmGcEmitter::new();
        emitter.emit();

        // Verify type indices are set
        assert_ne!(emitter.node_base_type, 0);
        assert_ne!(emitter.empty_type, 0);
        assert_ne!(emitter.number_type, 0);
        assert_ne!(emitter.codepoint_type, 0);
        assert_ne!(emitter.text_type, 0);
    }

    #[test]
    fn test_generate_wasm() {
        let mut emitter = WasmGcEmitter::new();
        emitter.emit();
        let bytes = emitter.finish();

        // Should have WASM magic number
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]);
        // Should have version 1
        assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_node_kind_enum() {
        assert_eq!(NodeKind::Empty as u32, 0);
        assert_eq!(NodeKind::Number as u32, 1);
        assert_eq!(NodeKind::Codepoint as u32, 3);
    }
}
