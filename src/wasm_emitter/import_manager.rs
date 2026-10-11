//! Import management for WASM modules

use crate::context::Context;
use crate::function::Function as FuncDef;
use crate::wasm_emitter::config::EmitterConfig;
use crate::wasm_emitter::type_manager::TypeManager;
use wasm_encoder::*;

/// An FFI import being emitted: module, name, a signature label, parameter and result types
type FfiImport = (String, String, &'static str, Vec<ValType>, Vec<ValType>);

/// Manages WASM imports: host functions, WASI functions, FFI functions
pub struct ImportManager {
	/// Import section for WASM module
	imports: ImportSection,

	/// Next available function index (tracks both imports and code functions)
	next_func_idx: u32,
}

impl Default for ImportManager {
	fn default() -> Self {
		Self::new()
	}
}

impl ImportManager {
	/// Create a new import manager
	pub fn new() -> Self {
		Self {
			imports: ImportSection::new(),
			next_func_idx: 0,
		}
	}

	/// Emit all imports based on configuration
	/// Must be called before emit_gc_types() since type indices need to be correct
	pub fn emit_imports(
		&mut self,
		config: &EmitterConfig,
		type_manager: &mut TypeManager,
		ctx: &mut Context,
	) {
		// Host imports (fetch, run)
		if config.emit_host_imports {
			self.emit_host_imports(type_manager, ctx);
		}

		// WASI imports (fd_write)
		if config.emit_wasi_imports {
			self.emit_wasi_imports(type_manager, ctx);
		}

		// FFI imports (libc, libm)
		if config.emit_ffi_imports && !ctx.ffi_imports.is_empty() {
			self.emit_ffi_imports(type_manager, ctx);
		}

		// Update next_func_idx to account for imports
		self.next_func_idx = ctx.func_registry.import_count();
	}

	/// An import that is no function (`env.memory`, `env.table`): function indices stay as they are
	pub fn import_entity(&mut self, module: &str, name: &str, entity: EntityType) {
		self.imports.import(module, name, entity);
	}

	/// Emit host function imports: fetch(url) -> string, run(wasm) -> i64
	fn emit_host_imports(&mut self, type_manager: &mut TypeManager, ctx: &mut Context) {
		// (ptr, len) -> (result_ptr, result_len): fetch's url and read's path; a negative length marks a failure reason
		let fetch_type_idx = type_manager.function_type(vec![ValType::I32, ValType::I32], vec![ValType::I32, ValType::I32]);
		// (wasm_ptr, wasm_len) -> the result value
		let run_type_idx = type_manager.function_type(vec![ValType::I32, ValType::I32], vec![ValType::I64]);
		self.import_function(ctx, "host", "fetch", fetch_type_idx, "host_fetch");
		// fetch_within: (url_ptr, url_len, timeout_ms: i64) -> (result_ptr, result_len), for an explicit timeout
		let fetch_within_type_idx = type_manager.function_type(
			vec![ValType::I32, ValType::I32, ValType::I64],
			vec![ValType::I32, ValType::I32],
		);
		self.import_function(ctx, "host", "fetch_within", fetch_within_type_idx, "host_fetch_within");
		self.import_function(ctx, "host", "read", fetch_type_idx, "host_read");
		// warn: (message_ptr, message_len) -> (), a runtime warning (diagnostic::report_runtime_warning)
		let warn_type_idx = type_manager.function_type(vec![ValType::I32, ValType::I32], vec![]);
		self.import_function(ctx, "host", "warn", warn_type_idx, "host_warn");
		self.import_function(ctx, "host", "run", run_type_idx, "host_run");
	}

	/// Emit WASI imports: fd_write(fd, iovs, iovs_len, nwritten) -> errno
	fn emit_wasi_imports(&mut self, type_manager: &mut TypeManager, ctx: &mut Context) {
		let fd_write_type_idx = type_manager.function_type(vec![ValType::I32; 4], vec![ValType::I32]);
		self.import_function(ctx, "wasi_snapshot_preview1", "fd_write", fd_write_type_idx, "wasi_fd_write");
		if ctx.required_functions.contains(super::wasi_emitter::WASI_ENVIRONMENT) {
			let sizes_type = type_manager.function_type(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
			for (name, registered) in super::wasi_emitter::ENVIRON_IMPORTS.into_iter().zip(["wasi_environ_sizes_get", "wasi_environ_get"]) {
				self.import_function(ctx, "wasi_snapshot_preview1", name, sizes_type, registered);
			}
		}
	}

	/// Emit FFI imports for all registered FFI functions, from their library module ("m" for libm, "c" for libc)
	fn emit_ffi_imports(&mut self, type_manager: &mut TypeManager, ctx: &mut Context) {
		let mut imports: Vec<FfiImport> = ctx
			.ffi_imports
			.iter()
			// the linear words are the module's own functions (linear_arrays.rs)
			.filter(|(_, sig)| sig.library != super::linear_arrays::LINEAR_LIBRARY)
			.map(|(name, sig)| (name.clone(), sig.library.to_string(), sig.name, sig.params.clone(), sig.results.clone()))
			.collect();
		imports.sort_by(|(a, ..), (b, ..)| a.cmp(b));

		for (name, library, import_name, params, results) in imports {
			let type_idx = type_manager.function_type(params, results);
			let registered_name: &'static str = Box::leak(format!("ffi_{}", name).into_boxed_str());
			self.import_function(ctx, &library, import_name, type_idx, registered_name);
		}
	}

	/// Import `module.name` of type `type_idx`, known to the function registry as `registered_name`
	fn import_function(&mut self, ctx: &mut Context, module: &str, name: &str, type_idx: u32, registered_name: &'static str) {
		self.imports.import(module, name, EntityType::Function(type_idx));
		ctx.func_registry.register(FuncDef::host(registered_name));
	}

	/// Get the import section (for adding to WASM module)
	pub fn imports(&self) -> &ImportSection {
		&self.imports
	}

	/// Get the import count
	pub fn import_count(&self) -> u32 {
		self.next_func_idx
	}

	/// Check if imports section is empty
	pub fn is_empty(&self) -> bool {
		self.next_func_idx == 0
	}
}
