//! Finishing a module: validation, the name section (every name WASM can carry) and the functions left unused
use super::*;

impl WasmGcEmitter {
	#[cfg(not(feature = "validate"))]
	pub(super) fn try_validate_wasm(_bytes: &[u8]) -> Result<(), String> {
		Ok(()) // the engine that runs the module validates it (the browser build, web/playground)
	}

	#[cfg(feature = "validate")]
	pub(super) fn try_validate_wasm(bytes: &[u8]) -> Result<(), String> {
		let mut features = WasmFeatures::default();
		features.set(WasmFeatures::REFERENCE_TYPES, true);
		features.set(WasmFeatures::GC, true);
		features.set(WasmFeatures::EXCEPTIONS, true);
		let mut validator = Validator::new_with_features(features);
		match validator.validate_all(bytes) {
			Ok(_) => {
				trace!("✓ WASM validation with GC features passed");
				Ok(())
			}
			Err(e) => Err(format!("{}", e)),
		}
	}

	/// The module bytes; a module that fails validation is a compiler bug, only test helpers may treat it as a panic
	pub fn finish(self) -> Vec<u8> {
		self.try_finish().unwrap_or_else(|message| panic!("{message}"))
	}

	/// The program's file as given, for a program playing files; none for an absolute path, which would carry this
	/// machine's folders into the module
	fn program_file_for_the_host(&self) -> Option<String> {
		let plays_files = [crate::host::FILE_NAMING_STD_MODULE, crate::host::ANY_STD_MODULE]
			.iter()
			.any(|module| self.std_modules.contains(*module));
		let file = crate::modules::program_file().filter(|file| plays_files && file.is_relative())?;
		Some(file.to_string_lossy().into_owned())
	}

	/// The module's own memory, at least as many pages as its texts take
	fn memory_section(&self) -> MemorySection {
		let mut section = MemorySection::new();
		if self.owns_memory {
			let pages = (self.string_table.next_offset() as u64).div_ceil(PAGE_BYTES).max(MEMORY.minimum);
			section.memory(MemoryType { minimum: pages, ..MEMORY });
		}
		section
	}

	/// The module bytes, or the validation failure as an error message
	pub fn try_finish(mut self) -> Result<Vec<u8>, String> {
		// WASM section order: types, imports, functions, memory, globals, exports, start, elements, code, data, names
		self.module.section(self.type_manager.types());
		if self.ctx.func_registry.import_count() > 0 || !self.ctx.imported_entities.is_empty() {
			self.module.section(self.import_manager.imports());
		}
		self.module.section(&self.functions);
		self.module.section(&self.memory_section());
		if !self.tags.is_empty() {
			self.module.section(&self.tags);
		}
		if self.next_global_idx > 0 {
			self.module.section(&self.globals);
		}
		self.module.section(self.exports.section());
		if let Some(function_index) = self.start_index {
			self.module.section(&StartSection { function_index });
		}
		// functions taken by ref.func: one declarative element segment for all (a module has one element section)
		let declared_functions: Vec<u32> = self
			.closures
			.declared_functions()
			.into_iter()
			.chain(self.witness_dispatchers())
			.chain(self.late_called_functions())
			.collect();
		if !declared_functions.is_empty() {
			let mut elements = ElementSection::new();
			elements.declared(Elements::Functions(declared_functions.into()));
			self.module.section(&elements);
		}
		self.module.section(&self.code);
		self.module.section(self.string_table.data_section());
		let c_calls = crate::modules::wasm_modules::c_calls(self.ctx.ffi_imports.values());
		if !c_calls.is_empty() {
			self.module.section(&CustomSection {
				name: crate::modules::wasm_modules::C_CALLS_SECTION.into(),
				data: c_calls.as_bytes().into(),
			});
		}
		if !self.std_modules.is_empty() {
			let modules = self.std_modules.iter().cloned().collect::<Vec<_>>().join(crate::host::STD_MODULES_SEPARATOR);
			self.module.section(&CustomSection {
				name: crate::host::STD_MODULES_SECTION.into(),
				data: modules.as_bytes().into(),
			});
		}
		if let Some(file) = self.program_file_for_the_host() {
			self.module.section(&CustomSection {
				name: crate::host::PROGRAM_FILE_SECTION.into(),
				data: file.as_bytes().into(),
			});
		}
		if crate::compiler::compile_context::read(|context| context.counts_fuel) {
			self.module.section(&CustomSection { name: crate::host::FUEL_SECTION.into(), data: [].as_slice().into() });
		}
		if !self.live_loops.is_empty() {
			let names = self.live_loops.iter().cloned().collect::<Vec<_>>().join(crate::host::USER_FUNCTIONS_SEPARATOR);
			self.module.section(&CustomSection {
				name: crate::host::LIVE_LOOPS_SECTION.into(),
				data: names.as_bytes().into(),
			});
		}
		let user_functions: Vec<&str> = (self.ctx.user_functions.iter())
			.filter(|(_, definition)| definition.func_index.is_some())
			.map(|(name, _)| name.as_str())
			.collect();
		if !user_functions.is_empty() {
			self.module.section(&CustomSection {
				name: crate::host::USER_FUNCTIONS_SECTION.into(),
				data: user_functions.join(crate::host::USER_FUNCTIONS_SEPARATOR).as_bytes().into(),
			});
		}
		self.emit_names();
		self.module.section(&self.names);

		let bytes = crate::compiler::dead_functions::without_dead_functions(&self.module.finish())
			.map_err(|message| format!("internal error: dropping dead functions failed: {message}"))?;
		write_debug_module(&bytes);
		Self::try_validate_wasm(&bytes)
			.map_err(|message| format!("internal error: WASM validation failed: {message}"))?;
		Ok(bytes)
	}

	/// The name section: subsections in id order (module, functions, types, globals, fields, tags), every map by index
	pub(super) fn emit_names(&mut self) {
		self.names.module("warp_compact");

		let functions: Vec<(u32, String)> = self
			.ctx
			.func_registry
			.all()
			.iter()
			.map(|f| (f.call_index as u32, f.name.clone()))
			.chain(
				self.ctx
					.user_functions
					.values()
					.filter_map(|function| Some((function.func_index?, function.name.clone()))),
			)
			.chain(self.closures.entry_names())
			.chain(self.tuple_packers.iter().map(|(function, index)| (*index, crate::tuples::packer_name(function))))
			.chain(self.main_index.map(|index| (index, "main".to_string())))
			.chain(self.start_index.map(|index| (index, super::START_FUNCTION.to_string())))
			.collect();
		self.names.functions(&name_map(&mut functions.iter().map(|(idx, name)| (*idx, name.as_str())).collect()));

		// a user function's parameter names, for debuggers and wasm-tools; a page leaves them out for size, an importer
		// reads them from warp.meta (wasm_modules.rs read_exports)
		if !super::config::compile_mode().for_a_page {
			let parameter_names = self
				.ctx
				.user_functions
				.values()
				.filter_map(|function| {
					Some((
						function.func_index?,
						function.params.iter().enumerate().map(|(i, param)| (i as u32, param.name.as_str())).collect(),
					))
				})
				.collect();
			self.names.locals(&indirect_name_map(parameter_names));
		}

		let tm = &self.type_manager;
		let mut types = vec![
			(tm.string_type, "String"),
			(tm.i64_box_type, "i64box"),
			(tm.f64_box_type, "f64box"),
			(tm.node_type, "Node"),
			(tm.int_array_type, "IntArray"),
			(tm.int_list_type, "IntList"),
			(tm.float_array_type, "FloatArray"),
			(tm.float_list_type, "FloatList"),
		];
		types.extend(tm.meta_node_type.map(|index| (index, "MetaNode")));
		types.extend(self.ctx.user_type_indices.iter().map(|(name, idx)| (*idx, name.as_str())));
		let instance_names: Vec<(u32, String)> = self
			.instance_types
			.iter()
			.map(|(class, instance)| (instance.type_index, format!("{class}{}", maps::struct_backend::INSTANCE_SUFFIX)))
			.collect();
		types.extend(instance_names.iter().map(|(idx, name)| (*idx, name.as_str())));
		self.names.types(&name_map(&mut types));

		if self.next_global_idx > 0 {
			const KIND_GLOBALS: [&str; 12] = [
				"kind_empty",
				"kind_int",
				"kind_float",
				"kind_text",
				"kind_codepoint",
				"kind_symbol",
				"kind_key",
				"kind_block",
				"kind_list",
				"kind_data",
				"kind_meta",
				"kind_error",
			];
			let mut globals: Vec<(u32, &str)> = KIND_GLOBALS
				.iter()
				.enumerate()
				.filter(|(idx, _)| (*idx as u32) < self.ctx.kind_global_indices.len() as u32)
				.map(|(idx, name)| (idx as u32, *name))
				.collect();
			globals.extend(self.extra_global_names.iter().map(|(index, name)| (*index, name.as_str())));
			self.names.globals(&name_map(&mut globals));
		}

		let mut fields: Vec<(u32, Vec<(u32, &str)>)> = vec![
			(tm.node_type, vec![(0, "kind"), (1, "data"), (2, "value")]),
			(tm.string_type, vec![(0, "ptr"), (1, "len")]),
			(tm.i64_box_type, vec![(0, "value")]),
			(tm.f64_box_type, vec![(0, "value")]),
			(tm.int_list_type, vec![(0, "length"), (1, "items")]),
			(tm.float_list_type, vec![(0, "length"), (1, "items")]),
		];
		if let Some(meta_node_type) = tm.meta_node_type {
			let meta_fields = [(3, "line"), (4, "column"), (5, "comment")];
			fields.push((meta_node_type, fields[0].1.iter().copied().chain(meta_fields).collect()));
		}
		for type_def in self.ctx.type_registry.types() {
			if let Some(&type_idx) = self.ctx.user_type_indices.get(&type_def.name) {
				fields.push((
					type_idx,
					type_def.fields.iter().enumerate().map(|(i, field)| (i as u32, field.name.as_str())).collect(),
				));
			}
		}
		fields.extend(self.instance_types.values().map(|instance| {
			(
				instance.type_index,
				instance.fields.iter().enumerate().map(|(i, (field, _))| (i as u32, field.as_str())).collect(),
			)
		}));
		self.names.fields(&indirect_name_map(fields));

		if let Some(tag_names) = self.error_tag_names() {
			self.names.tags(&tag_names);
		}
	}

	pub fn get_unused_functions(&self) -> Vec<String> {
		self.ctx
			.func_registry
			.all()
			.iter()
			.map(|f| f.name.clone())
			.filter(|name: &String| !self.ctx.used_functions.contains(name.as_str()))
			.collect()
	}

	pub fn get_used_functions(&self) -> Vec<&'static str> {
		self.ctx.used_functions.iter().copied().collect()
	}
}

/// A name map in index order (the name section requires it), one name per index
pub(super) fn name_map(names: &mut Vec<(u32, &str)>) -> NameMap {
	names.sort_by_key(|(idx, _)| *idx);
	names.dedup_by_key(|(idx, _)| *idx);
	let mut map = NameMap::new();
	for (idx, name) in names.iter() {
		map.append(*idx, name);
	}
	map
}

/// Name maps of several functions or types, in index order, one map per index
pub(super) fn indirect_name_map(mut entries: Vec<(u32, Vec<(u32, &str)>)>) -> IndirectNameMap {
	entries.sort_by_key(|(idx, _)| *idx);
	entries.dedup_by_key(|(idx, _)| *idx);
	let mut map = IndirectNameMap::new();
	for (idx, mut names) in entries {
		map.append(idx, &name_map(&mut names));
	}
	map
}
