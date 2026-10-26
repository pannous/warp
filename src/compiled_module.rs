//! A compiled module and how it runs: its bytes and the host capabilities its imports need, linked and run by
//! wasmtime natively, by the embedding host in the browser build. Whoever compiled it (compiler.wasm, the Rust
//! pipeline) hands one of these to the harness (card rust-bootstrap: harness, not compiler).
use crate::node::Node;

/// A compiled program and the host capabilities its imports need.
#[cfg_attr(not(feature = "native"), allow(dead_code))] // the browser host links every import itself
#[derive(Clone)]
pub struct CompiledModule {
	pub bytes: Vec<u8>,
	pub(crate) needs_host: bool,
	pub(crate) needs_wasi: bool,
	pub(crate) needs_ffi: bool,
}

/// Run a compiled program with the linker its imports need
#[cfg(feature = "native")]
pub fn run_module(CompiledModule { bytes, needs_host, needs_wasi, needs_ffi }: CompiledModule) -> Node {
	use crate::run::wasm_reader::{read_bytes_with_imports, Imports};
	let result = crate::compiler::phase_times::timed("run (with wasmtime compile)", || {
		if needs_ffi || needs_wasi || needs_host {
			read_bytes_with_imports(&bytes, Imports { host: needs_host, wasi: needs_wasi, ffi: needs_ffi })
		} else {
			crate::run::wasm_reader::read_bytes(&bytes)
		}
	});
	result.unwrap_or_else(crate::run::failures::failed_run)
}

/// Without wasmtime the embedding host runs the program (the browser playground: web.rs); a final quantity's unit is
/// read back from the module's `warp.meta` section (entry units), as wasm_reader does natively
#[cfg(not(feature = "native"))]
pub fn run_module(module: CompiledModule) -> Node {
	crate::numbers::units::static_units::with_module_units(&module.bytes, crate::web::run_in_host(&module.bytes))
}
