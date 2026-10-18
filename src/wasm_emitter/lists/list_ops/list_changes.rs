//! Lists changed: concatenated, extended, the last item dropped, rebuilt, with an item at an index
use super::*;

impl WasmGcEmitter {
	/// list_concat(a, b): a new list, the cells of b copied and those of a in front of them; iteratively (copy_cells).
	/// Lists are shared (P200b), so `xs + ys` shares no cell with either: an item added to ys later is not in it
	pub(super) fn emit_list_concat(&mut self) {
		if !self.should_emit_function("list_concat") {
			return;
		}
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_ref = Ref(self.node_ref(false));
		let params = vec![node_ref_nullable, node_ref_nullable];
		self.runtime_function("list_concat", params, vec![node_ref], self.copy_cells_locals(), |s, f| {
			let (first, second) = (0, 1);
			let copy = CopyCells::at(2);
			for list in [second, first] {
				s.emit_empty_as_null(f, list);
				s.emit_collect_cells(f, list, None, &copy);
				s.emit_rebuild_cells(f, &copy);
			}
			// two empty lists: ø, never null (`[1][0..0] + []` trapped at its caller's ref.as_non_null, card traps-wasm)
			Self::emit_list(f, &[I::Block(BlockType::Result(node_ref)), I::LocalGet(copy.result), I::BrOnNonNull(0)]);
			s.call(f, "new_empty");
			f.instruction(&I::End);
		});
	}

	/// list_extend(list, items): `xs += ys`, what `xs.add(v)` lowers to: copies of the items' cells added to the end of
	/// the list in place, so every holder of it sees them (P200b, Python's extend); ø (an Empty node) becomes the first
	/// cell itself. Gives back the list, the copied items for a missing one (null), list_concat of a non-list
	pub(super) fn emit_list_extend(&mut self) {
		if !self.should_emit_function(LIST_EXTEND) {
			return;
		}
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_ref = Ref(self.node_ref(false));
		let node_type = self.type_manager.node_type;
		let set = |field_index: u32| I::StructSet { struct_type_index: node_type, field_index };
		let locals = [self.copy_cells_locals(), vec![ValType::I64, node_ref_nullable]].concat();
		self.runtime_function(
			LIST_EXTEND,
			vec![node_ref_nullable, node_ref_nullable],
			vec![node_ref],
			locals,
			|s, f| {
				let (list, items) = (0, 1);
				let copy = CopyCells::at(2);
				let kind = copy.result + 1;
				let walk_start = kind + 1;
				let extended_last = s.list_cursor(CursorPart::Extended).zip(s.list_cursor(CursorPart::Last));
				// the list extended last starts its walk to the end at its last cell then
				Self::emit_list(f, &[I::LocalGet(list), I::LocalSet(walk_start)]);
				if let Some((extended, last)) = extended_last {
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::GlobalGet(extended), I::LocalGet(list), I::RefEq, I::If(BlockType::Empty),
							I::GlobalGet(last), I::LocalSet(walk_start),
						I::End,
					]);
				}
				// items added at the end leave each cell at its index: the index cursor stays, and the count of another
				// list, so a loop over a list appending to this one walks on (a comprehension over a table's rows)
				Self::emit_list(f, &s.forget_cursor_parts(&[CursorPart::Extended]));
				if let Some(counted) = s.list_cursor(CursorPart::Counted) {
					Self::emit_list(f, &[I::GlobalGet(counted), I::LocalGet(list), I::RefEq, I::If(BlockType::Empty)]);
					Self::emit_list(f, &s.forget_cursor_parts(&[CursorPart::Counted]));
					f.instruction(&I::End);
				}
				s.emit_item_check(f, super::super::super::declared_values::LIST_ITEMS_CHECK, list, items);
				s.emit_empty_as_null(f, items);
				s.emit_collect_cells(f, items, None, &copy);
				s.emit_rebuild_cells(f, &copy);
				// no items: the list itself, ø for a missing one
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(copy.result), I::RefIsNull, I::If(BlockType::Empty),
						I::LocalGet(list), I::RefIsNull, I::If(BlockType::Empty),
				]);
				s.call(f, "new_empty");
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::Return,
					I::End,
					I::LocalGet(list), I::RefAsNonNull, I::Return,
					I::End,
					I::LocalGet(list), I::RefIsNull, I::If(BlockType::Empty),
						I::LocalGet(copy.result), I::RefAsNonNull, I::Return,
					I::End,
				]);
				s.emit_field(f, list, 0);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind),
					I::LocalGet(kind), I::I64Const(Kind::Empty as i64), I::I64Eq, I::If(BlockType::Empty),
				]);
				for field_index in 0..3 {
					f.instruction(&I::LocalGet(list));
					s.emit_field(f, copy.result, field_index);
					if field_index == 0 {
						s.emit_or_element_mark(f, list);
					}
					f.instruction(&set(field_index));
				}
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(list), I::RefAsNonNull, I::Return,
					I::End,
					I::LocalGet(kind), I::I64Const(Kind::List as i64), I::I64Ne, I::If(BlockType::Empty),
						I::LocalGet(list), I::LocalGet(items),
				]);
				s.call(f, "list_concat");
				Self::emit_list(f, &[I::Return, I::End]);
				// the last cell takes the copies as its rest, whose last cell is the list's last then
				let walk_to_last = |s: &Self, f: &mut Function| {
					Self::emit_list(
						f,
						&[I::LocalSet(copy.cell), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)],
					);
					s.emit_field(f, copy.cell, 2);
					Self::emit_list(f, &[I::RefIsNull, I::BrIf(1)]);
					s.emit_field(f, copy.cell, 2);
					Self::emit_list(f, &[I::LocalSet(copy.cell), I::Br(0), I::End, I::End]);
				};
				f.instruction(&I::LocalGet(walk_start));
				walk_to_last(s, f);
				Self::emit_list(f, &[I::LocalGet(copy.cell), I::LocalGet(copy.result), set(2)]);
				if let Some((extended, last)) = extended_last {
					f.instruction(&I::LocalGet(copy.result));
					walk_to_last(s, f);
					Self::emit_list(
						f,
						&[I::LocalGet(list), I::GlobalSet(extended), I::LocalGet(copy.cell), I::GlobalSet(last)],
					);
				}
				Self::emit_list(f, &[I::LocalGet(list), I::RefAsNonNull]);
			},
		);
	}

	/// list_become(list, fresh): the list takes the items of the fresh list in place, every holder sees them (`xs.pop(i)`,
	/// `xs.clear()`, `xs.insert(v, at: i)`, card aliased-list): its first cell takes fresh's kind, item and rest, its
	/// element mark stays and checks fresh's items; an empty fresh list makes it ø itself, ø takes nothing and is fresh
	pub(super) fn emit_list_become(&mut self) {
		if !self.should_emit_function(crate::list_mutations::LIST_BECOME) {
			return;
		}
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_ref = Ref(self.node_ref(false));
		let node_type = self.type_manager.node_type;
		let set = |field_index: u32| I::StructSet { struct_type_index: node_type, field_index };
		self.runtime_function(
			crate::list_mutations::LIST_BECOME,
			vec![node_ref_nullable, node_ref_nullable],
			vec![node_ref],
			vec![],
			|s, f| {
				Self::emit_list(f, &s.forget_list_cursor());
				let (list, fresh) = (0, 1);
				s.emit_item_check(f, super::super::super::declared_values::LIST_ITEMS_CHECK, list, fresh);
				s.emit_empty_as_null(f, fresh);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(list), I::RefIsNull, I::If(BlockType::Empty),
						I::LocalGet(fresh), I::RefIsNull, I::If(BlockType::Empty),
				]);
				s.call(f, "new_empty");
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::Return,
					I::End,
					I::LocalGet(fresh), I::RefAsNonNull, I::Return,
					I::End,
					I::LocalGet(fresh), I::RefIsNull, I::If(BlockType::Empty),
				]);
				s.emit_become_empty(f, list);
				Self::emit_list(f, &[I::LocalGet(list), I::RefAsNonNull, I::Return, I::End, I::LocalGet(list)]);
				s.emit_field(f, fresh, 0);
				s.emit_unmarked(f);
				s.emit_or_element_mark(f, list);
				Self::emit_list(f, &[set(0), I::LocalGet(list)]);
				s.emit_field(f, fresh, 1);
				Self::emit_list(f, &[set(1), I::LocalGet(list)]);
				s.emit_field(f, fresh, 2);
				Self::emit_list(f, &[set(2), I::LocalGet(list), I::RefAsNonNull]);
			},
		);
	}

	/// element_children(children, tail): an element's `{…}` with each `[…]` among them spliced in, recursively, as its
	/// own children in front of tail: `ul{ h2 [[li li]] }` is `ul{ h2 li li }` (card ul-li-fruit); empty items vanish
	pub(super) fn emit_element_children(&mut self) {
		if !self.should_emit_function(ELEMENT_CHILDREN) {
			return;
		}
		let node = self.type_manager.node_type;
		let node_ref_nullable = Ref(self.node_ref(true));
		let own_index = self.next_func_idx;
		let (children, tail, item) = (0, 1, 2);
		self.runtime_function(
			ELEMENT_CHILDREN,
			vec![node_ref_nullable, node_ref_nullable],
			vec![node_ref_nullable],
			vec![node_ref_nullable],
			|s, f| {
				let kind_is = |s: &Self, f: &mut Function, local: u32, kind: i64| {
					s.emit_field(f, local, 0);
					Self::emit_list(
						f,
						&[I::I64Const(crate::type_kinds::UNMARKED_KIND), I::I64And, I::I64Const(kind), I::I64Eq],
					);
				};
				// no children (null or ø): the tail
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(children), I::RefIsNull, I::If(BlockType::Empty),
						I::LocalGet(tail), I::Return,
					I::End,
				]);
				kind_is(s, f, children, Kind::Empty as i64);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(tail), I::Return, I::End]);
				// the rest first, so each cell is built once
				s.emit_field(f, children, 2);
				Self::emit_list(f, &[I::LocalGet(tail), I::Call(own_index), I::LocalSet(tail)]);
				s.emit_field(f, children, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::RefCastNullable(HeapType::Concrete(node)), I::LocalSet(item),
					I::LocalGet(item), I::RefIsNull, I::If(BlockType::Empty),
						I::LocalGet(tail), I::Return,
					I::End,
				]);
				kind_is(s, f, item, Kind::Empty as i64);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(tail), I::Return, I::End]);
				kind_is(s, f, item, SQUARE_LIST_KIND);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::If(BlockType::Empty),
						I::LocalGet(item), I::LocalGet(tail), I::Call(own_index), I::Return,
					I::End,
					I::LocalGet(item), I::LocalGet(tail),
					I::I64Const(super::super::super::bracket_info(&crate::node::Bracket::Curly)),
				]);
				s.call(f, "new_list");
			},
		);
	}

	/// element_body(children): the spliced children as a literal element holds them: ø for none, the child itself for
	/// one (`ul:h2:"Fruit"`), else the list
	pub(super) fn emit_element_body(&mut self) {
		if !self.should_emit_function(ELEMENT_BODY) {
			return;
		}
		let node = self.type_manager.node_type;
		let (node_ref, node_ref_nullable) = (Ref(self.node_ref(false)), Ref(self.node_ref(true)));
		let (children, spliced) = (0, 1);
		self.runtime_function(
			ELEMENT_BODY,
			vec![node_ref_nullable],
			vec![node_ref],
			vec![node_ref_nullable],
			|s, f| {
				Self::emit_list(f, &[I::LocalGet(children), I::RefNull(HeapType::Concrete(node))]);
				s.call(f, ELEMENT_CHILDREN);
				Self::emit_list(f, &[I::LocalTee(spliced), I::RefIsNull, I::If(BlockType::Empty)]);
				s.call(f, "new_empty");
				Self::emit_list(f, &[I::Return, I::End]);
				s.emit_field(f, spliced, 2);
				Self::emit_list(f, &[I::RefIsNull, I::If(BlockType::Empty)]);
				s.emit_field(f, spliced, 1);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::RefCastNonNull(HeapType::Concrete(node)), I::Return,
					I::End,
					I::LocalGet(spliced), I::RefAsNonNull,
				]);
			},
		);
	}

	/// The locals copy_cells needs, in CopyCells order
	pub(in crate::wasm_emitter) fn copy_cells_locals(&self) -> Vec<ValType> {
		let node_ref_nullable = Ref(self.node_ref(true));
		let cells = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.node_array_type) });
		vec![node_ref_nullable, cells, ValType::I32, node_ref_nullable, node_ref_nullable]
	}

	/// The first cells of `list` (all, or as many as the i64 local `limit` says; a negative limit is all) into an array,
	/// counted, then filled: no recursion, so a long list never exhausts the call stack. `cell` ends at the first cell
	/// not taken
	pub(in crate::wasm_emitter) fn emit_collect_cells(
		&self,
		f: &mut Function,
		list: u32,
		limit: Option<u32>,
		copy: &CopyCells,
	) {
		let node_array = self.type_manager.node_array_type;
		let reached_limit = |f: &mut Function| {
			if let Some(limit) = limit {
				Self::emit_list(
					f,
					&[I::LocalGet(copy.position), I::I64ExtendI32U, I::LocalGet(limit), I::I64Eq, I::BrIf(1)],
				);
			}
		};
		let next_cell = |s: &Self, f: &mut Function| {
			s.emit_field(f, copy.cell, 2);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(copy.cell),
				I::LocalGet(copy.position), I::I32Const(1), I::I32Add, I::LocalSet(copy.position),
				I::Br(0),
				I::End,
				I::End,
			]);
		};
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(list), I::LocalSet(copy.cell),
			I::I32Const(0), I::LocalSet(copy.position),
			I::Block(BlockType::Empty),
				I::Loop(BlockType::Empty),
					I::LocalGet(copy.cell), I::RefIsNull, I::BrIf(1),
		]);
		reached_limit(f);
		next_cell(self, f);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(copy.position), I::ArrayNewDefault(node_array), I::LocalSet(copy.cells),
			I::LocalGet(list), I::LocalSet(copy.cell),
			I::I32Const(0), I::LocalSet(copy.position),
			I::Block(BlockType::Empty),
				I::Loop(BlockType::Empty),
					I::LocalGet(copy.position), I::LocalGet(copy.cells), I::ArrayLen, I::I32GeU, I::BrIf(1),
					I::LocalGet(copy.cells), I::LocalGet(copy.position), I::LocalGet(copy.cell),
					I::ArraySet(node_array),
		]);
		next_cell(self, f);
	}

	/// The collected cells, from the last back, copied in front of `result`, each sharing what follows it
	pub(in crate::wasm_emitter) fn emit_rebuild_cells(&self, f: &mut Function, copy: &CopyCells) {
		let node_array = self.type_manager.node_array_type;
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::Block(BlockType::Empty),
				I::Loop(BlockType::Empty),
					I::LocalGet(copy.position), I::I32Eqz, I::BrIf(1),
					I::LocalGet(copy.position), I::I32Const(1), I::I32Sub, I::LocalSet(copy.position),
					I::LocalGet(copy.cells), I::LocalGet(copy.position), I::ArrayGet(node_array),
					I::LocalSet(copy.copied),
		]);
		self.emit_field(f, copy.copied, 0);
		self.emit_field(f, copy.copied, 1);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(copy.result), I::StructNew(self.type_manager.node_type), I::LocalSet(copy.result),
			I::Br(0),
			I::End,
			I::End,
		]);
	}

	/// `x#i += v` → `x#i = x#i + v`, leaving the assigned value (i64) on the stack
	pub(in crate::wasm_emitter) fn emit_compound_index_assignment(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &crate::operators::Op,
		right: &Node,
	) -> bool {
		let Node::Key(target, crate::operators::Op::Hash, index) = left.drop_meta() else {
			return false;
		};
		let updated = Node::Key(Box::new(left.clone()), op.base_op(), Box::new(right.clone()));
		self.emit_index_assignment(func, target, index, &updated);
		true
	}

	/// Index assignment: `y#i=v` stores `node_with_at(y, i, v)` back into `y`; a list changes in place (P200b), a text,
	/// which never changes, is a new one
	pub(super) fn emit_with_at_functions(&mut self) {
		if !self.should_emit_function("node_with_at") {
			return;
		}
		self.emit_text_heap_global();
		let node_ref = Ref(self.node_ref(false));
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let string_type = self.type_manager.string_type;
		let next_index = |s: &Self| s.ctx.func_registry.import_count() + s.ctx.func_registry.code_count();

		// list_with_at(list, index, value node): the cell at index takes the value in place, so every holder of the list sees
		// it (P200b); the list itself is given back
		let list_with_at = next_index(self);
		let params = vec![node_ref_nullable, ValType::I64, node_ref];
		let locals = vec![node_ref_nullable, ValType::I64];
		self.runtime_function("list_with_at", params, vec![node_ref], locals, |s, f| {
			let (list, index, value, cell, countdown) = (0, 1, 2, 3, 4);
			let kind_test = |s: &Self, f: &mut Function| {
				// a missing cell or ø: lists do not grow by index, the empty list ø included (user, 2026-10-04)
				Self::emit_list(f, &[I::LocalGet(cell), I::RefIsNull]);
				s.emit_fail_if(f, "index_out_of_range");
				s.emit_field(f, cell, 0);
				Self::emit_kind_test(f, Kind::Empty as i64, I::I64Eq);
				s.emit_fail_if(f, "index_out_of_range");
			};
			Self::emit_index_compare(f, I::I64LtS);
			s.emit_fail_if(f, "index_out_of_range");
			s.emit_item_check(f, super::super::super::declared_values::LIST_ITEM_CHECK, list, value);
			// the cell at index
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(list), I::LocalSet(cell),
				I::LocalGet(index), I::LocalSet(countdown),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
			]);
			kind_test(s, f);
			Self::emit_list(f, &[I::LocalGet(countdown), I::I64Const(1), I::I64Eq, I::BrIf(1)]);
			s.emit_field(f, cell, 2);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(cell),
				I::LocalGet(countdown), I::I64Const(1), I::I64Sub, I::LocalSet(countdown),
				I::Br(0),
				I::End,
				I::End,
			]);
			// it takes the value, in place: an entry replaced, so no map index holds the old one
			Self::emit_list(f, &s.forget_map_indexes());
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(cell), I::LocalGet(value), I::StructSet { struct_type_index: node_type, field_index: 1 },
				I::LocalGet(list), I::RefAsNonNull,
			]);
		});
		assert_eq!(self.func_index("list_with_at"), list_with_at, "node_with_at forwards to this index");

		// text_with_char_at(text, index, value): a fresh copy with the grapheme at index replaced by the UTF-8 of value
		let text_with_char_at = next_index(self);
		let params = vec![node_ref, ValType::I64, ValType::I64];
		self.runtime_function("text_with_char_at", params.clone(), vec![node_ref], vec![ValType::I32; 8], |s, f| {
			let (start, stop, pointer, end, width, length, copy, codepoint) = (3, 4, 5, 6, 7, 8, 9, 10);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "text_grapheme_offset");
			f.instruction(&I::LocalSet(start));
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &[I::LocalGet(start), I::LocalGet(end)]);
			s.call(f, "grapheme_end");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(stop),
				I::LocalGet(2), I::I32WrapI64, I::LocalTee(codepoint), I32Const(utf8::MAX_CODE_POINT), I::I32GtU,
			]);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_utf8_width(f, codepoint);
			Self::emit_list(
				f,
				&[
					I::LocalSet(width),
					// length = bytes - replaced grapheme + width
					I::LocalGet(end),
					I::LocalGet(pointer),
					I::I32Sub,
					I::LocalGet(stop),
					I::LocalGet(start),
					I::I32Sub,
					I::I32Sub,
					I::LocalGet(width),
					I::I32Add,
					I::LocalSet(length),
				],
			);
			s.emit_text_allocation(f, length, copy);
			let copy_bytes = I::MemoryCopy { src_mem: 0, dst_mem: 0 };
			Self::emit_list(
				f,
				&[
					// bytes before the grapheme
					I::LocalGet(copy),
					I::LocalGet(pointer),
					I::LocalGet(start),
					I::LocalGet(pointer),
					I::I32Sub,
					copy_bytes.clone(),
					// start becomes the destination of the new character, the rest follows it
					I::LocalGet(copy),
					I::LocalGet(start),
					I::I32Add,
					I::LocalGet(pointer),
					I::I32Sub,
					I::LocalTee(start),
					I::LocalGet(width),
					I::I32Add,
					I::LocalGet(stop),
					I::LocalGet(end),
					I::LocalGet(stop),
					I::I32Sub,
					copy_bytes,
				],
			);
			Self::emit_utf8_store(f, start, codepoint, width, stop);
			s.emit_field(f, 0, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(copy), I::LocalGet(length), I::StructNew(string_type),
				I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type),
			]);
		});

		// node_with_at(node, index, value): texts and symbols by grapheme, everything else as list
		self.runtime_function("node_with_at", params, vec![node_ref], vec![], |s, f| {
			s.emit_is_text(f);
			f.instruction(&I::If(BlockType::Result(node_ref)));
			s.emit_forward_arguments(f, text_with_char_at, None);
			f.instruction(&I::Else);
			s.emit_forward_arguments(f, list_with_at, Some("new_int"));
			f.instruction(&I::End);
		});
		// node_with_node_at(node, index, value node): as node_with_at, with any value in a list (a float stays a float)
		if self.should_emit_function(NODE_WITH_NODE_AT) {
			self.runtime_function(
				NODE_WITH_NODE_AT,
				vec![node_ref, ValType::I64, node_ref],
				vec![node_ref],
				vec![],
				|s, f| {
					s.emit_is_text(f);
					f.instruction(&I::If(BlockType::Result(node_ref)));
					s.emit_forward_arguments(f, text_with_char_at, Some("get_int_value"));
					f.instruction(&I::Else);
					s.emit_forward_arguments(f, list_with_at, None);
					f.instruction(&I::End);
				},
			);
		}
	}
}

/// list_extend(list, items): `xs += ys` in place (emit_list_extend)
pub const LIST_EXTEND: &str = "list_extend";
