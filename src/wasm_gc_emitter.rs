use crate::node::{Node, Kind, Bracket, DataType};
use crate::extensions::numbers::Number;
use wasm_encoder::*;

/// WebAssembly GC emitter for Node AST
/// Generates WASM GC bytecode with functions for working with serialized nodes
pub struct WasmGcEmitter {
    module: Module,
    types: TypeSection,
    functions: FunctionSection,
    code: CodeSection,
    exports: ExportSection,
    next_type_idx: u32,
    next_func_idx: u32,
}

/// Node variant tags (discriminant values)
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeTag {
    Empty = 0,
    Number = 1,
    Text = 2,
    Symbol = 3,
    KeyValue = 4,
    Pair = 5,
    Tag = 6,
    Block = 7,
    List = 8,
    Data = 9,
    WithMeta = 10,
}

impl WasmGcEmitter {
    pub fn new() -> Self {
        WasmGcEmitter {
            module: Module::new(),
            types: TypeSection::new(),
            functions: FunctionSection::new(),
            code: CodeSection::new(),
            exports: ExportSection::new(),
            next_type_idx: 0,
            next_func_idx: 0,
        }
    }

    /// Generate all type definitions and functions
    pub fn emit(&mut self) {
        self.emit_core_functions();
    }

    fn emit_core_functions(&mut self) {
        // make_empty() -> i32
        // Returns tag for Empty node
        self.types.ty().function(vec![], vec![ValType::I32]);
        let make_empty_idx = self.next_type_idx;
        self.next_type_idx += 1;

        let mut func = Function::new(vec![]);
        func.instruction(&Instruction::I32Const(NodeTag::Empty as i32));
        func.instruction(&Instruction::End);

        self.functions.function(make_empty_idx);
        self.code.function(&func);
        self.exports.export("make_empty", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // make_int(i64) -> i32
        // Returns tag for Number node
        self.types.ty().function(vec![ValType::I64], vec![ValType::I32]);
        let make_int_idx = self.next_type_idx;
        self.next_type_idx += 1;

        let mut func = Function::new(vec![(1, ValType::I64)]);
        func.instruction(&Instruction::I32Const(NodeTag::Number as i32));
        func.instruction(&Instruction::End);

        self.functions.function(make_int_idx);
        self.code.function(&func);
        self.exports.export("make_int", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;

        // make_float(f64) -> i32
        self.types.ty().function(vec![ValType::F64], vec![ValType::I32]);
        let make_float_idx = self.next_type_idx;
        self.next_type_idx += 1;

        let mut func = Function::new(vec![(1, ValType::F64)]);
        func.instruction(&Instruction::I32Const(NodeTag::Number as i32));
        func.instruction(&Instruction::End);

        self.functions.function(make_float_idx);
        self.code.function(&func);
        self.exports.export("make_float", ExportKind::Func, self.next_func_idx);
        self.next_func_idx += 1;
    }

    /// Build the module and return WASM bytes
    pub fn build(mut self) -> Vec<u8> {
        self.module.section(&self.types);
        self.module.section(&self.functions);
        self.module.section(&self.exports);
        self.module.section(&self.code);
        self.module.finish()
    }

    /// Write WASM module to file
    pub fn emit_to_file(&self, path: &str) -> std::io::Result<()> {
        // Need to clone self to call build()
        let mut emitter = WasmGcEmitter {
            module: Module::new(),
            types: TypeSection::new(),
            functions: FunctionSection::new(),
            code: CodeSection::new(),
            exports: ExportSection::new(),
            next_type_idx: self.next_type_idx,
            next_func_idx: self.next_func_idx,
        };
        emitter.emit();
        let bytes = emitter.build();

        use std::fs::File;
        use std::io::Write;
        let mut file = File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }
}

impl Default for WasmGcEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_basic_module() {
        let mut emitter = WasmGcEmitter::new();
        emitter.emit();
        let bytes = emitter.build();

        assert!(!bytes.is_empty());
        // WASM magic number: 0x00 0x61 0x73 0x6D
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
    }

    #[test]
    fn test_emit_to_file() {
        let mut emitter = WasmGcEmitter::new();
        emitter.emit();

        let result = emitter.emit_to_file("/tmp/test-nodes.wasm");
        assert!(result.is_ok());

        // Verify file was created
        let bytes = std::fs::read("/tmp/test-nodes.wasm").unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
    }

    #[test]
    fn test_node_tags() {
        assert_eq!(NodeTag::Empty as u32, 0);
        assert_eq!(NodeTag::Number as u32, 1);
        assert_eq!(NodeTag::Text as u32, 2);
        assert_eq!(NodeTag::WithMeta as u32, 10);
    }
}
