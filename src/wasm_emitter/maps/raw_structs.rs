//! Raw structs: a standalone module returning a GC struct instance of a user type, and running it
use super::super::*;

impl WasmGcEmitter {
	/// Emit a standalone WASM module that returns a raw GC struct instance.
	/// This is the standard path for returning user-defined GC objects.
	///
	/// # Arguments
	/// * `type_def` - The type definition for the struct
	/// * `field_values` - Field values as (name, RawFieldValue) pairs
	///
	/// # Returns
	/// WASM bytes that can be loaded and executed to get the GC struct
	pub fn emit_raw_struct(type_def: &TypeDef, field_values: &[RawFieldValue]) -> Vec<u8> {
		use wasm_encoder::StorageType::Val;
		use wasm_encoder::*;

		let mut module = Module::new();

		// Collect string data and compute offsets
		let mut string_data = Vec::new();
		let mut string_offsets: Vec<(usize, usize)> = Vec::new(); // (offset, len) for each string field
		let mut current_offset = 0usize;

		for value in field_values {
			if let RawFieldValue::String(s) = value {
				string_offsets.push((current_offset, s.len()));
				string_data.extend_from_slice(s.as_bytes());
				current_offset += s.len();
			} else {
				string_offsets.push((0, 0)); // placeholder for non-strings
			}
		}

		let mut types = TypeSection::new();

		// Type 0: $String = struct { ptr: i32, len: i32 }
		types.ty().struct_(type_manager::string_fields());
		let string_type_idx = 0u32;

		// Type 1: User struct type
		let string_ref = RefType { nullable: false, heap_type: HeapType::Concrete(string_type_idx) };
		let struct_fields: Vec<FieldType> = type_def
			.fields
			.iter()
			.map(|f| {
				let element_type = match crate::type_kinds::field_storage(&f.type_name) {
					crate::type_kinds::FieldStorage::I64 => Val(ValType::I64),
					crate::type_kinds::FieldStorage::I32 => Val(ValType::I32),
					crate::type_kinds::FieldStorage::F64 => Val(ValType::F64),
					crate::type_kinds::FieldStorage::F32 => Val(ValType::F32),
					crate::type_kinds::FieldStorage::Text => Val(Ref(string_ref)),
					_ => Val(ValType::I64), // default
				};
				FieldType { element_type, mutable: false }
			})
			.collect();
		types.ty().struct_(struct_fields);
		let struct_type_idx = 1u32;

		// Type 2: main() -> ref $StructType
		let struct_ref = RefType { nullable: false, heap_type: HeapType::Concrete(struct_type_idx) };
		types.ty().func_type(&FuncType::new([], [Ref(struct_ref)]));

		module.section(&types);

		// Function section
		let mut functions = FunctionSection::new();
		functions.function(2); // main uses type 2
		module.section(&functions);

		// Memory section (only if we have strings)
		let has_strings = !string_data.is_empty();
		if has_strings {
			let mut memories = MemorySection::new();
			memories.memory(MemoryType {
				minimum: 1,
				maximum: None,
				memory64: false,
				shared: false,
				page_size_log2: None,
			});
			module.section(&memories);
		}

		// Export section
		let mut exports = ExportSection::new();
		if has_strings {
			exports.export("memory", ExportKind::Memory, 0);
		}
		exports.export("main", ExportKind::Func, 0);
		module.section(&exports);

		// Code section
		let mut codes = CodeSection::new();
		let mut func = Function::new([]);

		// Emit field values in order
		let mut string_idx = 0usize;
		for value in field_values.iter() {
			match value {
				RawFieldValue::I64(v) => {
					func.instruction(&I::I64Const(*v));
				}
				RawFieldValue::I32(v) => {
					func.instruction(&I32Const(*v));
				}
				RawFieldValue::F64(v) => {
					func.instruction(&I::F64Const(Ieee64::new(v.to_bits())));
				}
				RawFieldValue::F32(v) => {
					func.instruction(&I::F32Const(Ieee32::new(v.to_bits())));
				}
				RawFieldValue::String(_) => {
					let (ptr, len) = string_offsets[string_idx];
					Self::emit_list(
						&mut func,
						&[I32Const(ptr as i32), I32Const(len as i32), I::StructNew(string_type_idx)],
					);
					string_idx += 1;
				}
			}
		}
		Self::emit_list(&mut func, &[I::StructNew(struct_type_idx), I::End]);
		codes.function(&func);
		module.section(&codes);

		// Data section for strings
		if has_strings {
			let mut data = DataSection::new();
			data.active(0, &ConstExpr::i32_const(0), string_data.iter().copied());
			module.section(&data);
		}

		// Name section for field name resolution, subsections in ascending id order (function 1, type 4, field 10)
		let mut names = NameSection::new();

		// Function names (subsection 1 comes before types and fields)
		let mut func_names = NameMap::new();
		func_names.append(0, "main");
		names.functions(&func_names);

		// Type names
		let mut type_names = NameMap::new();
		type_names.append(string_type_idx, "String");
		type_names.append(struct_type_idx, &type_def.name);
		names.types(&type_names);

		// Field names for structs
		let mut type_field_names = IndirectNameMap::new();

		// String struct fields
		Self::append_string_field_names(&mut type_field_names, string_type_idx);

		// User struct fields
		let mut struct_fields_names = NameMap::new();
		for (i, field) in type_def.fields.iter().enumerate() {
			struct_fields_names.append(i as u32, &field.name);
		}
		type_field_names.append(struct_type_idx, &struct_fields_names);
		names.fields(&type_field_names);

		module.section(&names);

		let bytes = module.finish();
		write_debug_module(&bytes);
		bytes
	}
}

#[cfg(feature = "native")]
/// Run raw struct WASM and return GcObject wrapped in Node::Data
pub fn run_raw_struct(wasm_bytes: &[u8]) -> Result<Node, String> {
	use wasmtime::{Linker, Val};

	// Register WASM metadata for field name lookup in Debug output
	let module_id = crate::gc_traits::register_gc_types_from_wasm(wasm_bytes).ok();

	let engine = gc_engine();
	let mut store = crate::util::limited_store(&engine, ());
	let module =
		crate::run::module_cache::compiled_module(&engine, wasm_bytes).map_err(|e: wasmtime::Error| e.to_string())?;

	let linker = Linker::new(&engine);
	let instance = linker.instantiate(&mut store, &module).map_err(|e: wasmtime::Error| e.to_string())?;

	let main = instance.get_func(&mut store, "main").ok_or_else(|| "no main function".to_string())?;

	let mut results = vec![Val::I32(0)];
	main.call(&mut store, &[], &mut results).map_err(|e: wasmtime::Error| e.to_string())?;

	let gc_obj = ErgonomicGcObject::new(results[0], store, Some(instance))
		.map_err(|e: anyhow::Error| e.to_string())?
		.with_module(module_id);

	Ok(crate::node::data(gc_obj))
}

/// Find a struct instantiation anywhere in the AST using TypeRegistry
/// todo instead of recursing different types individually, we should have one central walker and delegate from there.
/// The instance a program consists of: type definitions, then `TypeName:{field:value, ...}` as its value. An instance
/// anywhere else (an argument `f(pt{…})`, a statement among others) is no such program: it runs the standard way
pub(crate) fn find_struct_instantiation(registry: &TypeRegistry, node: &Node) -> Option<(TypeDef, Vec<RawFieldValue>)> {
	let statements = match node.drop_meta() {
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => items.as_slice(),
		_ => std::slice::from_ref(node),
	};
	let (last, before) = statements.split_last()?;
	if !before.iter().all(|statement| matches!(statement.drop_meta(), Node::Type { .. } | Node::Empty)) {
		return None;
	}
	find_instantiation_recursive(registry, last)
}

pub(in crate::wasm_emitter) fn find_instantiation_recursive(
	registry: &TypeRegistry,
	node: &Node,
) -> Option<(TypeDef, Vec<RawFieldValue>)> {
	/// todo instead of recursing different types individually, we should have one central walker and delegate from there.
	use crate::type_kinds::extract_instance_values;
	let node = node.drop_meta();
	match node {
		Node::Key(left, Op::Colon, right) => {
			if let Node::Symbol(name) = left.drop_meta() {
				if let Some(type_def) = registry.get_by_name(name) {
					if let Some((_, values)) = extract_instance_values(node) {
						return Some((type_def.clone(), values));
					}
				}
			}
			find_instantiation_recursive(registry, right)
		}
		Node::List(items, _, _) => items.iter().find_map(|item| find_instantiation_recursive(registry, item)),
		_ => None,
	}
}
