//! Import management for WASM modules

use crate::context::Context;
use crate::function::Function as FuncDef;
use crate::wasm_emitter::config::EmitterConfig;
use crate::wasm_emitter::type_manager::TypeManager;
use std::collections::HashMap;
use wasm_encoder::*;

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

	/// Emit host function imports: fetch(url) -> string, run(wasm) -> i64
	fn emit_host_imports(&mut self, type_manager: &mut TypeManager, ctx: &mut Context) {
		// Type for fetch: (i32, i32) -> (i32, i32)
		// Takes (url_ptr, url_len), returns (result_ptr, result_len)
		let fetch_type_idx = type_manager.add_function_type(
			vec![ValType::I32, ValType::I32],
			vec![ValType::I32, ValType::I32],
		);

		// Type for run: (i32, i32) -> i64
		// Takes (wasm_ptr, wasm_len), returns result value
		let run_type_idx = type_manager.add_function_type(
			vec![ValType::I32, ValType::I32],
			vec![ValType::I64],
		);

		// Import fetch from "host" module
		self.imports
			.import("host", "fetch", EntityType::Function(fetch_type_idx));
		Self::register_import(ctx, "host_fetch");

		// Import run from "host" module
		self.imports
			.import("host", "run", EntityType::Function(run_type_idx));
		Self::register_import(ctx, "host_run");
	}

	/// Emit WASI imports (fd_write from wasi_snapshot_preview1)
	fn emit_wasi_imports(&mut self, type_manager: &mut TypeManager, ctx: &mut Context) {
		// Type for fd_write: (fd: i32, iovs: i32, iovs_len: i32, nwritten: i32) -> i32
		let fd_write_type_idx = type_manager.add_function_type(
			vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
			vec![ValType::I32],
		);

		// Import fd_write from wasi_snapshot_preview1
		self.imports.import(
			"wasi_snapshot_preview1",
			"fd_write",
			EntityType::Function(fd_write_type_idx),
		);
		Self::register_import(ctx, "wasi_fd_write");
	}

	/// Emit FFI imports for all registered FFI functions
	fn emit_ffi_imports(&mut self, type_manager: &mut TypeManager, ctx: &mut Context) {
		// Clone data to avoid borrow conflict
		let mut imports: Vec<(String, String, Vec<ValType>, Vec<ValType>)> = ctx
			.ffi_imports
			.iter()
			.map(|(name, sig)| {
				(
					name.clone(),
					sig.library.to_string(),
					sig.params.clone(),
					sig.results.clone(),
				)
			})
			.collect();
		imports.sort_by(|(a, _, _, _), (b, _, _, _)| a.cmp(b));

		for (name, library, params, results) in imports {
			let type_idx = type_manager.add_function_type(params, results);

			// Import from library module (e.g., "m" for libm, "c" for libc)
			self.imports
				.import(&library, &name, EntityType::Function(type_idx));

			// Register as an import function - use a leaked string for static lifetime
			let static_name: &'static str = Box::leak(format!("ffi_{}", name).into_boxed_str());
			Self::register_import(ctx, static_name);
		}
	}

	/// Register an import function in the context
	fn register_import(ctx: &mut Context, name: &'static str) -> u32 {
		let func = FuncDef::host(name);
		ctx.func_registry.register(func)
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
