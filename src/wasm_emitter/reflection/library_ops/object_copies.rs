//! Objects: instance_copy, and a field set or added in place (field_set_in_place, fields_grow, field_with)
use super::*;

impl WasmGcEmitter {
	/// instance_copy(node, shallow): a copy of any object, keeping its class (P205/P207). Deep unless `shallow` is true:
	/// every instance, map, list and entry inside is new, each copied once however often it is held (cycles too);
	/// numbers, texts and resources (closures, data) are shared, they never change. Shallow: the object's own entries
	/// and cells are new, the values in them shared
	pub(in crate::wasm_emitter) fn emit_instance_copy(&mut self) {
		if !self.should_emit_function(crate::library_words::INSTANCE_COPY) {
			return;
		}
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		let node_heap = HeapType::Concrete(node_type);
		let has_instances = !self.ctx.type_registry.types().is_empty();
		let next_index = |s: &Self| s.next_func_idx;
		// is the node in `local` an entry or cell (a Key or List), else leave the function giving it back
		let return_unless_structure = |f: &mut Function, local: u32, kind: u32| {
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(local), I::StructGet { struct_type_index: node_type, field_index: 0 },
				I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(KEY_KIND), I::I64Ne,
				I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::I32And, I::LocalGet(kind),
				I::I64Const(Kind::Empty as i64), I::I64Ne, I::I32And, I::If(BlockType::Empty),
					I::LocalGet(local), I::Return,
				I::End,
			]);
		};
		let entry_copy = |s: &Self, f: &mut Function, local: u32| {
			for field_index in 0..3 {
				s.emit_field(f, local, field_index);
			}
			f.instruction(&I::StructNew(node_type));
		};

		// copy_spine(fields): new entries and cells, the values in them shared
		let spine = next_index(self);
		self.runtime_function(
			"copy_spine",
			vec![nullable],
			vec![nullable],
			vec![ValType::I64, Self::any_ref(), nullable],
			|s, f| {
				let (node, kind, item, entry) = (0, 1, 2, 3);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(node), I::RefIsNull, I::If(BlockType::Empty),
						I::RefNull(node_heap), I::Return,
					I::End,
				]);
				return_unless_structure(f, node, kind);
				Self::emit_list(
					f,
					&[I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::If(BlockType::Empty)],
				);
				entry_copy(s, f, node);
				Self::emit_list(f, &[I::Return, I::End]);
				// a cell: its item new when it is an entry
				s.emit_field(f, node, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalTee(item), I::RefTestNonNull(node_heap), I::If(BlockType::Empty),
						I::LocalGet(item), I::RefCastNonNull(node_heap), I::LocalTee(entry),
						I::StructGet { struct_type_index: node_type, field_index: 0 }, I::I64Const(KIND_MASK),
						I::I64And, I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty),
				]);
				entry_copy(s, f, entry);
				Self::emit_list(f, &[I::LocalSet(item), I::End, I::End]);
				s.emit_field(f, node, 0);
				f.instruction(&I::LocalGet(item));
				s.emit_field(f, node, 2);
				Self::emit_list(f, &[I::Call(spine), I::StructNew(node_type)]);
			},
		);
		let spine = self.func_index("copy_spine"); // the copy_spine emitted before, when the runtime emitters run again

		// deep_copy(node, memo): memo.value lists `original:copy` pairs, so a part held twice is copied once
		let deep = next_index(self);
		self.runtime_function(
			"deep_copy",
			vec![nullable, node_ref],
			vec![nullable],
			vec![ValType::I64, nullable, nullable, nullable, Self::any_ref()],
			|s, f| {
				let (node, memo, kind, cell, pair, copy, data) = (0, 1, 2, 3, 4, 5, 6);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(node), I::RefIsNull, I::If(BlockType::Empty),
						I::RefNull(node_heap), I::Return,
					I::End,
				]);
				return_unless_structure(f, node, kind);
				s.emit_field(f, memo, 2);
				f.instruction(&I::LocalSet(cell));
				s.emit_cell_walk(f, cell, |f| {
					s.emit_field(f, cell, 1);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::RefCastNonNull(node_heap), I::LocalTee(pair),
						I::StructGet { struct_type_index: node_type, field_index: 1 }, I::RefCastNonNull(node_heap),
						I::LocalGet(node), I::RefEq, I::If(BlockType::Empty),
					]);
					s.emit_field(f, pair, 2);
					Self::emit_list(f, &[I::Return, I::End]);
				});
				s.emit_field(f, node, 0);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::RefNull(HeapType::Abstract { shared: false, ty: AbstractHeapType::Any }), I::RefNull(node_heap),
					I::StructNew(node_type), I::LocalSet(copy),
					I::LocalGet(memo), I::I64Const(Kind::List as i64), I::I64Const(KEY_KIND), I::LocalGet(node),
					I::LocalGet(copy), I::StructNew(node_type),
				]);
				s.emit_field(f, memo, 2);
				Self::emit_list(
					f,
					&[I::StructNew(node_type), I::StructSet { struct_type_index: node_type, field_index: 2 }],
				);
				s.emit_field(f, node, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(data),
					I::LocalGet(copy), I::LocalGet(data), I::RefTestNonNull(node_heap),
					I::If(BlockType::Result(Self::any_ref())),
						I::LocalGet(data), I::RefCastNonNull(node_heap), I::LocalGet(memo), I::Call(deep),
					I::Else,
						I::LocalGet(data),
					I::End,
					I::StructSet { struct_type_index: node_type, field_index: 1 },
					I::LocalGet(copy),
				]);
				s.emit_field(f, node, 2);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(memo), I::Call(deep), I::StructSet { struct_type_index: node_type, field_index: 2 },
					I::LocalGet(copy),
				]);
			},
		);
		let deep = self.func_index("deep_copy");

		self.runtime_function(
			crate::library_words::INSTANCE_COPY,
			vec![node_ref, node_ref],
			vec![node_ref],
			vec![nullable],
			|s, f| {
				let (object, shallow, body) = (0, 1, 2);
				f.instruction(&I::LocalGet(shallow));
				s.call(f, super::super::super::reflection::equality::IS_TRUTHY);
				f.instruction(&I::If(BlockType::Empty));
				if has_instances {
					f.instruction(&I::LocalGet(object));
					s.call(f, super::super::super::lists::list_ops::STRUCT_BODY);
					Self::emit_list(
						f,
						&[I::LocalTee(body), I::LocalGet(object), I::RefEq, I::I32Eqz, I::If(BlockType::Empty)],
					);
					s.emit_field(f, object, 0);
					s.emit_field(f, object, 1);
					Self::emit_list(
						f,
						&[I::LocalGet(body), I::Call(spine), I::StructNew(node_type), I::Return, I::End],
					);
				}
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(object), I::Call(spine), I::RefAsNonNull, I::Return,
					I::End,
					I::LocalGet(object), I::I64Const(Kind::Empty as i64),
					I::RefNull(HeapType::Abstract { shared: false, ty: AbstractHeapType::Any }), I::RefNull(node_heap),
					I::StructNew(node_type), I::Call(deep), I::RefAsNonNull,
				]);
			},
		);
	}

	/// field_set_in_place(fields, name, value) -> found: the entry `name:…` among an object's fields (one entry, or a
	/// cons list of them) gets the value in place
	pub(super) fn emit_field_set_in_place(&mut self) {
		let (node_ref, nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let node_type = self.type_manager.node_type;
		self.runtime_function(
			FIELD_SET_IN_PLACE,
			vec![nullable, node_ref, node_ref],
			vec![ValType::I32],
			vec![nullable, ValType::I64],
			|s, f| {
				Self::emit_list(f, &s.forget_list_cursor_keeping_map_indexes());
				let (cells, name, value, entry, kind) = (0, 1, 2, 3, 4);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(cells), I::RefIsNull, I::BrIf(1),
				]);
				s.emit_field(f, cells, 0);
				Self::emit_list(
					f,
					&[I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(KEY_KIND), I::I64Eq],
				);
				// one field: the fields are that entry; else the entry is the cell's item
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(cells), I::LocalSet(entry), I::Else]);
				s.emit_field(f, cells, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::RefCastNullable(HeapType::Concrete(node_type)), I::LocalSet(entry),
					I::End,
					I::LocalGet(entry), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty),
						I::LocalGet(entry), I::RefAsNonNull, I::LocalGet(name),
				]);
				s.call(f, "map_entry_has_key");
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::If(BlockType::Empty),
						I::LocalGet(entry), I::LocalGet(value),
						I::StructSet { struct_type_index: node_type, field_index: 2 },
						I32Const(1), I::Return,
					I::End,
					I::End,
					I::LocalGet(kind), I::I64Const(KEY_KIND), I::I64Eq, I::BrIf(1),
				]);
				s.emit_field(f, cells, 2);
				Self::emit_list(f, &[I::LocalSet(cells), I::Br(0), I::End, I::End, I32Const(0)]);
			},
		);
	}

	/// fields_grow(fields, entry): the entry added to an object's fields in place, so every holder sees it (P200b). The
	/// empty object becomes the entry itself, a single entry a cons list of the two; meta entries `@name:value` stay
	/// behind every field, so a walk by position never meets one
	pub(super) fn emit_fields_grow(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let nullable = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let set = |field_index: u32| I::StructSet { struct_type_index: node_type, field_index };
		self.runtime_function(FIELDS_GROW, vec![node_ref, node_ref], vec![], vec![ValType::I64, nullable], |s, f| {
			Self::emit_list(f, &s.forget_list_cursor_keeping_map_indexes());
			let (fields, entry, kind, cells) = (0, 1, 2, 3);
			Self::emit_list(f, &[I::LocalGet(fields), I::LocalGet(entry)]);
			s.call(f, super::super::super::maps::map_index::MAP_INDEX_GROW);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::Return, I::End]);
			s.emit_field(f, fields, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind),
				I::LocalGet(kind), I::I64Const(Kind::Empty as i64), I::I64Eq, I::If(BlockType::Empty),
			]);
			for field_index in 0..3 {
				f.instruction(&I::LocalGet(fields));
				s.emit_field(f, entry, field_index);
				f.instruction(&set(field_index));
			}
			Self::emit_list(f, &[I::Return, I::End]);
			// a single entry moves into a new first cell
			Self::emit_list(
				f,
				&[I::LocalGet(kind), I::I64Const(KEY_KIND), I::I64Eq, I::If(BlockType::Empty), I::LocalGet(fields)],
			);
			for field_index in 0..3 {
				s.emit_field(f, fields, field_index);
			}
			Self::emit_list(
				f,
				&[
					I::StructNew(node_type),
					set(1),
					I::LocalGet(fields),
					I::I64Const(CURLY_LIST_KIND),
					set(0),
					I::LocalGet(fields),
					I::RefNull(HeapType::Concrete(node_type)),
					set(2),
					I::I64Const(Kind::List as i64),
					I::LocalSet(kind),
					I::End,
					I::LocalGet(kind),
					I::I64Const(Kind::List as i64),
					I::I64Ne,
					I::If(BlockType::Empty),
				],
			);
			s.call(f, "not_an_object");
			Self::emit_list(f, &[I::End, I::LocalGet(fields), I::LocalSet(cells), I::Loop(BlockType::Empty)]);
			// in front of the first meta entry: the cell keeps the new field, a new cell after it the meta entry
			s.emit_field(f, cells, 1);
			s.call(f, super::super::super::reflection::equality::IS_META_ENTRY);
			f.instruction(&I::LocalGet(entry));
			s.call(f, super::super::super::reflection::equality::IS_META_ENTRY);
			Self::emit_list(f, &[I::I32Eqz, I::I32And, I::If(BlockType::Empty), I::LocalGet(cells)]);
			for field_index in 0..3 {
				s.emit_field(f, cells, field_index);
			}
			Self::emit_list(
				f,
				&[I::StructNew(node_type), set(2), I::LocalGet(cells), I::LocalGet(entry), set(1), I::Return, I::End],
			);
			// at the end: a new last cell
			s.emit_field(f, cells, 2);
			Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Empty), I::LocalGet(cells)]);
			s.emit_field(f, cells, 0);
			Self::emit_list(
				f,
				&[
					I::LocalGet(entry),
					I::RefNull(HeapType::Concrete(node_type)),
					I::StructNew(node_type),
					set(2),
					I::Return,
					I::End,
				],
			);
			s.emit_field(f, cells, 2);
			Self::emit_list(f, &[I::LocalSet(cells), I::Br(0), I::End]);
		});
	}

	/// field_with(object, name, value): the field `name` of the object set to `value` in place, added at the end when it
	/// is new; gives back the object itself (objects are references, P200/P200b). A one-entry object `{a:1}` is the entry
	/// node itself; an instance `T:{fields}` changes its fields.
	pub(in crate::wasm_emitter) fn emit_field_with(&mut self) {
		if !self.should_emit_function("field_with") {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let colon = crate::operators::op_to_code(&crate::operators::Op::Colon);
		self.emit_field_set_in_place();
		self.emit_fields_grow();
		let has_instances = !self.ctx.type_registry.types().is_empty();
		let nullable = Ref(self.node_ref(true));
		let locals = vec![node_ref, ValType::I64, nullable, ValType::I32];
		self.runtime_function("field_with", vec![node_ref, node_ref, node_ref], vec![node_ref], locals, |s, f| {
			let (object, name, value, fields, kind, entry, indexed) = (0, 1, 2, 3, 4, 5, 6);
			let add_field = |s: &mut Self, f: &mut Function| {
				Self::emit_list(f, &[I::LocalGet(fields), I::LocalGet(name), I::LocalGet(value), I::I64Const(colon)]);
				s.call(f, "new_key");
				f.instruction(&I::RefAsNonNull);
				s.call(f, FIELDS_GROW);
			};
			f.instruction(&I::LocalGet(object));
			if has_instances {
				s.call(f, super::super::super::lists::list_ops::STRUCT_BODY);
			}
			f.instruction(&I::LocalSet(fields));
			s.emit_field(f, fields, 0);
			Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind)]);
			for object_kind in [Kind::Empty, Kind::Key, Kind::List] {
				Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(object_kind as i64), I::I64Ne]);
			}
			Self::emit_list(f, &[I::I32And, I::I32And, I::If(BlockType::Empty)]);
			s.call(f, "not_an_object");
			f.instruction(&I::End);
			// the name as a symbol like the names of an object literal
			f.instruction(&I::LocalGet(name));
			s.call(f, super::super::super::lists::list_ops::MAP_KEY_NAME);
			f.instruction(&I::LocalSet(name));
			s.emit_require_kind(f, name, Kind::Symbol, "not_a_text");
			// a map walked lately has an index: its entry takes the value, else the field is new
			Self::emit_list(f, &[I::LocalGet(fields), I::LocalGet(name)]);
			s.call(f, super::super::super::maps::map_index::MAP_INDEX_ENTRY);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(indexed),
				I::LocalSet(entry),
				I::LocalGet(indexed), I::If(BlockType::Empty),
					I::LocalGet(entry), I::RefIsNull, I::If(BlockType::Empty),
			]);
			add_field(s, f);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Else,
					I::LocalGet(entry), I::LocalGet(value),
					I::StructSet { struct_type_index: s.type_manager.node_type, field_index: 2 },
				I::End,
				I::LocalGet(object), I::Return,
				I::End,
				I::LocalGet(fields), I::LocalGet(name), I::LocalGet(value),
			]);
			s.call(f, FIELD_SET_IN_PLACE);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			add_field(s, f);
			Self::emit_list(f, &[I::End, I::LocalGet(object)]);
		});
	}
}
