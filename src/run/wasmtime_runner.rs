use crate::node::Node;
use crate::type_kinds::KIND_MASK;
use crate::wasm_reader;

/// Run a compiled .wasm file; a failure is an error node
pub fn run(path: &str) -> Node {
	wasm_reader::run_wasm_gc_object(path).map(|result| Node::from_gc_object(&result)).unwrap_or_else(crate::wasm_emitter::failed_run)
}

/// Run WAT text (wasmtime compiles it like a binary module) with FFI imports; a failure is an error node
pub fn run_wat(wat_code: &str) -> Node {
	wasm_reader::read_bytes_with_ffi(wat_code.as_bytes()).unwrap_or_else(crate::wasm_emitter::failed_run)
}
