//! The runtime functions of the Node itself: its constructors and getters, user type constructors, math helpers
use super::super::*;

impl WasmGcEmitter {
	/// Emit with user-defined types from a TypeRegistry
	/// Order: memory, gc_types, user_types, kind_globals, constructors, user_constructors
	pub fn emit_with_types(&mut self, registry: &TypeRegistry) {
		// Memory
		self.emit_memory();

		// Core GC types (String, Node, i64box, f64box)
		self.emit_gc_types();

		// User-defined struct types (before any functions!)
		self.emit_user_types(registry);

		// Kind globals
		if self.config.emit_kind_globals {
			self.emit_kind_globals();
		}

		// Core Node constructors
		self.emit_constructors();

		// User type constructors
		self.emit_user_type_constructors(registry);
	}

	/// Emit constructor functions for user-defined types
	pub(in crate::wasm_emitter) fn emit_user_type_constructors(&mut self, registry: &TypeRegistry) {
		for type_def in registry.types() {
			self.emit_user_type_constructor(type_def);
		}
	}

	/// Emit a constructor function for a single user type: new_TypeName(fields...) -> ref $TypeName
	pub(in crate::wasm_emitter) fn emit_user_type_constructor(&mut self, type_def: &TypeDef) {
		let type_idx = match self.ctx.user_type_indices.get(&type_def.name) {
			Some(idx) => *idx,
			None => return,
		};

		let type_ref = RefType { nullable: false, heap_type: HeapType::Concrete(type_idx) };

		let params: Vec<ValType> = type_def.fields.iter().map(|f| field_def_to_val_type(f, self)).collect();
		let field_count = params.len() as u32;
		self.host_only_function(&format!("new_{}", type_def.name), params, vec![Ref(type_ref)], vec![], |_, func| {
			(0..field_count).for_each(|field| {
				func.instruction(&I::LocalGet(field));
			});
			func.instruction(&I::StructNew(type_idx));
		});
	}

	/// Emit constructor functions for the compact Node
	pub(in crate::wasm_emitter) fn emit_constructors(&mut self) {
		// Emit basic Node constructors using macros
		self.emit_int_heap_globals();
		self.emit_list_cursor_globals();
		self.emit_text_cursor_globals();
		self.emit_map_index_globals();
		self.emit_text_foundation();
		constructors::emit_all_constructors(self);
		self.emit_runtime_errors(); // before the int runtime: exact_div fails divide_by_zero
		self.emit_text_address();
		self.emit_int_runtime();
		self.export_exact_builders();
		// a C function returning char * hands its text in through the text heap (ffi.rs text_node)
		if self
			.ctx
			.ffi_imports
			.values()
			.any(|sig| sig.library != crate::host::HOST_LIBRARY && matches!(sig.results.first(), Some(ValType::Ref(_))))
		{
			self.emit_text_heap_global();
		}
		self.emit_getters(); // before the list ops (list_at calls get_int_value), after the runtime errors it calls
					   // Emit list and string operation functions
		self.emit_function_text(); // before the list ops: a function value joins as its text
		self.emit_list_ops();
		self.emit_linear_arrays();
		self.emit_cells();
		self.emit_text_of();
		self.emit_equality_ops();
		self.emit_node_type_name(); // after values_equal, which names an instance's declared type
		self.emit_text_as_int(); // after the getters: it calls get_int_value
		self.emit_text_as_float();
		self.emit_text_as_number(); // after text_as_float and text_as_int, which it calls
		self.emit_uncertain_runtime(); // after text_as_float, which it calls
		self.emit_times_runtime();
		self.emit_time_zones();
		self.emit_time_arithmetic();
		if self.config.emit_reflection {
			self.emit_reflection();
		}
		self.emit_math_helpers();
		self.emit_text_builtins();
		self.emit_map_get(); // after the text builtins: map keys are compared by text_of; with the hash tables
		self.emit_library_ops(); // after the text builtins: the library words call text_of
		self.emit_similarity_ops(); // after the library ops and the equality ops it calls
		self.emit_node_arithmetic(); // after text_as_float, get_int_value, text_concat and list_join, which it calls
		self.emit_node_api(); // after text_of, text_concat and error_of, which it calls
	}

	pub(in crate::wasm_emitter) fn emit_getters(&mut self) {
		let node_ref = self.node_ref(true);

		// get_kind(node: ref $Node) -> i64
		self.exported_function("get_kind", vec![Ref(node_ref)], vec![ValType::I64], vec![], |s, func| {
			Self::emit_list(
				func,
				&[I::LocalGet(0), I::StructGet { struct_type_index: s.type_manager.node_type, field_index: 0 }],
			);
		});

		// get_int_value(node: ref $Node) -> i64
		// Extract integer from Node's data field (i64box); a character is its code point ('5' as int is 53), so a
		// character variable compares with c >= '0'. Any other node is the runtime error not_an_int, which a `try`
		// catches, never a raw cast trap
		self.exported_function("get_int_value", vec![Ref(node_ref)], vec![ValType::I64], vec![], |s, func| {
			let node_kind_is = |kind: Kind| {
				[
					I::LocalGet(0),
					I::StructGet { struct_type_index: s.type_manager.node_type, field_index: 0 },
					I::I64Const(KIND_MASK),
					I::I64And,
					I::I64Const(kind as i64),
					I::I64Eq,
				]
			};
			Self::emit_list(func, &node_kind_is(Kind::Codepoint));
			Self::emit_list(func, &[I::If(BlockType::Empty), I::LocalGet(0)]);
			s.emit_codepoint_of_node(func);
			Self::emit_list(func, &[I::Return, I::End]);
			Self::emit_list(func, &node_kind_is(Kind::Int));
			Self::emit_list(func, &[I::I32Eqz, I::If(BlockType::Empty)]);
			s.emit_runtime_error(func, "not_an_int");
			Self::emit_list(func, &[I::End, I::LocalGet(0)]); // Node
			func.instruction(&I::StructGet {
				struct_type_index: s.type_manager.node_type,
				field_index: 1, // data field
			});
			s.emit_int_from_payload(func);
		});
		// get_text_ptr / get_text_len(node: ref $Node) -> i32: the $String of a Text, Symbol or Error, 0 for any other node,
		// so a host without GC field access (JavaScript) can read a result text from memory
		for (name, field_index) in [("get_text_ptr", 0), ("get_text_len", 1)] {
			self.exported_function(name, vec![Ref(node_ref)], vec![ValType::I32], vec![], |s, f| {
				let string = HeapType::Concrete(s.type_manager.text_type);
				s.emit_field(f, 0, 1);
				Self::emit_list(f, &[I::RefTestNonNull(string), I::If(BlockType::Result(ValType::I32))]);
				s.emit_text_field(f, 0, field_index);
				Self::emit_list(f, &[I::Else, I::I32Const(0), I::End]);
			});
		}
	}

	/// Emit math helper functions (i64_pow, etc.)
	pub(in crate::wasm_emitter) fn emit_math_helpers(&mut self) {
		// i64_pow(base, exponent) -> i64: base multiplied exponent times
		if self.should_emit_function("i64_pow") {
			self.exported_function(
				"i64_pow",
				vec![ValType::I64, ValType::I64],
				vec![ValType::I64],
				vec![ValType::I64],
				|_, func| {
					let (base, exponent, result) = (0, 1, 2);
					#[rustfmt::skip]
					Self::emit_list(func, &[
						I::I64Const(1), I::LocalSet(result),
						I::Block(BlockType::Empty),
							I::Loop(BlockType::Empty),
								I::LocalGet(exponent), I::I64Eqz, I::BrIf(1),
								I::LocalGet(result), I::LocalGet(base), I::I64Mul, I::LocalSet(result),
								I::LocalGet(exponent), I::I64Const(1), I::I64Sub, I::LocalSet(exponent),
								I::Br(0),
						I::End, I::End,
						I::LocalGet(result),
					]);
				},
			);
		}
	}
}
