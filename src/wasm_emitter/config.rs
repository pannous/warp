//! Configuration for WASM GC emitter

/// Configuration for the WASM GC emitter
#[derive(Debug, Clone)]
pub struct EmitterConfig {
	/// Emit all functions (if false, enables tree-shaking)
	pub emit_all_functions: bool,
	/// Emit Kind globals for documentation
	pub emit_kind_globals: bool,
	/// Emit host function imports (fetch, run)
	pub emit_host_imports: bool,
	/// Emit WASI imports (fd_write)
	pub emit_wasi_imports: bool,
	/// Emit FFI imports (libc, libm)
	pub emit_ffi_imports: bool,
	/// Export the reflection getters (reflection.rs) a host without GC field access needs: on for the browser build, a page
	/// warp builds and a module for any host (`warp compile --wasm`)
	pub emit_reflection: bool,
}

impl Default for EmitterConfig {
	fn default() -> Self {
		Self {
			emit_all_functions: true,
			emit_kind_globals: !crate::pipeline::is_for_a_page(), // a page's host reads kinds from get_kind, not these
			emit_host_imports: false,
			emit_wasi_imports: false,
			emit_ffi_imports: false,
			emit_reflection: cfg!(not(feature = "native")) || crate::pipeline::is_for_a_page() || crate::pipeline::is_for_any_host(),
		}
	}
}
