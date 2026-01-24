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
}

impl Default for EmitterConfig {
	fn default() -> Self {
		Self {
			emit_all_functions: true,
			emit_kind_globals: true,
			emit_host_imports: false,
			emit_wasi_imports: false,
			emit_ffi_imports: false,
		}
	}
}

impl EmitterConfig {
	/// Create a new builder for EmitterConfig
	pub fn builder() -> EmitterConfigBuilder {
		EmitterConfigBuilder::default()
	}
}

/// Builder for EmitterConfig
#[derive(Default)]
pub struct EmitterConfigBuilder {
	config: EmitterConfig,
}

impl EmitterConfigBuilder {
	/// Enable/disable tree-shaking (inverse of emit_all_functions)
	pub fn tree_shaking(mut self, enabled: bool) -> Self {
		self.config.emit_all_functions = !enabled;
		self
	}

	/// Enable/disable Kind globals
	pub fn kind_globals(mut self, enabled: bool) -> Self {
		self.config.emit_kind_globals = enabled;
		self
	}

	/// Enable/disable host imports
	pub fn host_imports(mut self, enabled: bool) -> Self {
		self.config.emit_host_imports = enabled;
		self
	}

	/// Enable/disable WASI imports
	pub fn wasi_imports(mut self, enabled: bool) -> Self {
		self.config.emit_wasi_imports = enabled;
		self
	}

	/// Enable/disable FFI imports
	pub fn ffi_imports(mut self, enabled: bool) -> Self {
		self.config.emit_ffi_imports = enabled;
		self
	}

	/// Build the config
	pub fn build(self) -> EmitterConfig {
		self.config
	}
}
