use crate::node::Node;
use crate::wasm_reader;

/// Run a compiled .wasm file (or its machine code, a .cwasm) with every import family linked, since a file does not
/// say which it needs; a failure is an error node
pub fn run(path: &str) -> Node {
	let every_import = wasm_reader::Imports { host: true, wasi: true, ffi: true };
	std::fs::read(path)
		.map_err(anyhow::Error::from)
		.and_then(|bytes| wasm_reader::read_bytes_with_imports(&bytes, every_import))
		.unwrap_or_else(crate::wasm_emitter::failed_run)
}

/// Run WAT text (wasmtime compiles it like a binary module) with FFI imports; a failure is an error node
pub fn run_wat(wat_code: &str) -> Node {
	wasm_reader::read_bytes_with_ffi(wat_code.as_bytes()).unwrap_or_else(crate::wasm_emitter::failed_run)
}
