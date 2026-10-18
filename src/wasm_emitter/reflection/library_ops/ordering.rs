//! The order of nodes: node_order and list_sort
use super::*;

impl WasmGcEmitter {
	/// node_order(a, b): negative, 0 or positive as a is before, with or after b. Numbers by value (as f64 when either is a
	/// float), texts and characters by code points (the order of their UTF-8 bytes), instances by the Comparable witness of
	/// their type (witness.rs); anything else is not comparable.
	pub(super) fn emit_node_order(&mut self) {
		let witness = self.compare_witness_table();
		let node_ref = Ref(self.node_ref(false));
		let float_box = self.type_manager.f64_box_type;
		let exact = self.int_runtime();
		let text_type = self.type_manager.text_type;
		let mut locals = vec![ValType::I64, ValType::I64, ValType::F64, ValType::F64, self.text_ref(), self.text_ref()];
		locals.extend([ValType::I32; 4]);
		self.runtime_function(NODE_ORDER, vec![node_ref, node_ref], vec![ValType::I32], locals, |s, f| {
			let (first, second, kinds) = (0, 1, [2, 3]);
			let (first_float, second_float) = (4, 5);
			let (first_bytes, second_bytes, first_length, second_length, index, difference) = (6, 7, 8, 9, 10, 11);
			let is_kind = |f: &mut Function, local: u32, kind: Kind| {
				Self::emit_list(f, &[I::LocalGet(local), I::I64Const(kind as i64), I::I64Eq])
			};
			let both = |f: &mut Function, kind: Kind, either: I<'static>| {
				is_kind(f, kinds[0], kind);
				is_kind(f, kinds[1], kind);
				f.instruction(&either);
			};
			s.emit_codepoint_as_text(f, first);
			s.emit_codepoint_as_text(f, second);
			s.emit_uncertain_as_value(f, first);
			s.emit_uncertain_as_value(f, second);
			for (node, kind) in [(first, kinds[0]), (second, kinds[1])] {
				s.emit_field(f, node, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And, I::LocalSet(kind)]);
			}
			// texts: the first differing byte, else the shorter one first
			both(f, Kind::Text, I::I32And);
			f.instruction(&I::If(BlockType::Empty));
			for (node, bytes, length) in [(first, first_bytes, first_length), (second, second_bytes, second_length)] {
				s.emit_text_bytes(f, node);
				Self::emit_list(f, &[I::LocalTee(bytes), I::ArrayLen, I::LocalSet(length)]);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(index), I::LocalGet(first_length), I::I32GeU, I::LocalGet(index),
						I::LocalGet(second_length), I::I32GeU, I::I32Or, I::BrIf(1),
						I::LocalGet(first_bytes), I::LocalGet(index), I::ArrayGetU(text_type),
						I::LocalGet(second_bytes), I::LocalGet(index), I::ArrayGetU(text_type), I::I32Sub,
						I::LocalTee(difference), I::If(BlockType::Empty),
							I::LocalGet(difference), I::Return,
						I::End,
						I::LocalGet(index), I32Const(1), I::I32Add, I::LocalSet(index),
						I::Br(0),
				I::End, I::End,
				I::LocalGet(first_length), I::LocalGet(second_length), I::I32Sub, I::Return,
				I::End,
			]);
			if let Some(table) = witness {
				s.emit_instance_order(f, table, kinds, first, second);
			}
			s.emit_time_order(f, kinds, first, second);
			// numbers only from here
			for kind in kinds {
				is_kind(f, kind, Kind::Int);
				is_kind(f, kind, Kind::Float);
				Self::emit_list(f, &[I::I32Or, I::I32Eqz]);
				s.emit_fail_if(f, "not_comparable");
			}
			both(f, Kind::Int, I::I32And);
			f.instruction(&I::If(BlockType::Empty));
			s.emit_int_value_of(f, first);
			s.emit_int_value_of(f, second);
			if exact {
				s.call(f, "exact_cmp");
			} else {
				// both kinds are known to be Int here, so their locals hold the values
				let (a, b) = (kinds[0], kinds[1]);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(b),
					I::LocalSet(a),
					I::LocalGet(a), I::LocalGet(b), I::I64GtS, I::LocalGet(a), I::LocalGet(b), I::I64LtS, I::I32Sub,
				]);
			}
			Self::emit_list(f, &[I::Return, I::End]);
			// a float on either side compares both as f64
			for (node, kind, float) in [(first, kinds[0], first_float), (second, kinds[1], second_float)] {
				is_kind(f, kind, Kind::Float);
				f.instruction(&I::If(BlockType::Result(ValType::F64)));
				s.emit_field(f, node, 1);
				Self::emit_list(
					f,
					&[
						I::RefCastNonNull(HeapType::Concrete(float_box)),
						I::StructGet { struct_type_index: float_box, field_index: 0 },
						I::Else,
					],
				);
				s.emit_int_value_of(f, node);
				if exact {
					s.call(f, "exact_to_f64");
				} else {
					f.instruction(&I::F64ConvertI64S);
				}
				Self::emit_list(f, &[I::End, I::LocalSet(float)]);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(first_float), I::LocalGet(second_float), I::F64Gt, I::LocalGet(first_float),
				I::LocalGet(second_float), I::F64Lt, I::I32Sub,
			]);
		});
	}

	/// list_sort(list): a new list of comparable values in ascending order (list_sorted_by the values themselves)
	pub(super) fn emit_list_sort(&mut self) {
		if !self.should_emit_function("list_sort") && !self.should_emit_function(LIST_SORTED_BY) {
			return;
		}
		let (nullable, node_ref) = (Ref(self.node_ref(true)), Ref(self.node_ref(false)));
		let (node_type, array) = (self.type_manager.node_type, self.type_manager.node_array_type);
		let array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let mut locals = vec![ValType::I32, ValType::I32, nullable];
		locals.extend([array_ref; 4]);
		locals.extend([ValType::I32; 7]);
		locals.extend([nullable, ValType::I32]);

		// list_sorted_by(list, keys): a new list of the list's items (a text's characters), ordered by their keys (the
		// items themselves when keys is null) as node_order compares them; a bottom-up merge sort over arrays, stable and
		// without recursion. `xs.sort_by(p => p.k)` calls it with the keys (lambdas.rs)
		self.runtime_function(LIST_SORTED_BY, vec![nullable, nullable], vec![node_ref], locals, |s, f| {
			let (list, keys, count, index, cell, cells, ranks, cells_to, ranks_to) = (0, 1, 2, 3, 4, 5, 6, 7, 8);
			let (width, low, middle, high, left, right, from, sorted, is_text) = (9, 10, 11, 12, 13, 14, 15, 16, 17);
			s.emit_text_as_characters(f, list, is_text);
			s.emit_empty_as_null(f, list);
			s.emit_require_list(f);
			let get = |array_local: u32, index_local: u32| {
				[I::LocalGet(array_local), I::LocalGet(index_local), I::ArrayGet(array)]
			};
			let increment = |local: u32| [I::LocalGet(local), I::I32Const(1), I::I32Add, I::LocalSet(local)];
			let rank_of_cell = |f: &mut Function| {
				Self::emit_list(f, &[I::LocalGet(ranks), I::LocalGet(index)]);
				s.emit_field(f, cell, 1);
				Self::emit_list(f, &[I::RefCastNullable(HeapType::Concrete(node_type)), I::ArraySet(array)]);
			};
			let smaller = |f: &mut Function, a: &[I<'static>], b: &[I<'static>]| {
				Self::emit_list(f, a);
				Self::emit_list(f, b);
				Self::emit_list(f, a);
				Self::emit_list(f, b);
				Self::emit_list(f, &[I::I32LtU, I::Select]);
			};
			Self::emit_list(f, &[I::LocalGet(list), I::LocalSet(cell)]);
			s.emit_cell_walk(f, cell, |f| Self::emit_list(f, &increment(count)));
			for array_local in [cells, ranks, cells_to, ranks_to] {
				Self::emit_list(f, &[I::LocalGet(count), I::ArrayNewDefault(array), I::LocalSet(array_local)]);
			}
			Self::emit_list(f, &[I::LocalGet(list), I::LocalSet(cell)]);
			s.emit_cell_walk(f, cell, |f| {
				Self::emit_list(f, &[I::LocalGet(cells), I::LocalGet(index), I::LocalGet(cell), I::ArraySet(array)]);
				rank_of_cell(f);
				Self::emit_list(f, &increment(index));
			});
			Self::emit_list(f, &[I::LocalGet(keys), I::LocalSet(cell), I::I32Const(0), I::LocalSet(index)]);
			s.emit_cell_walk(f, cell, |f| {
				Self::emit_list(f, &[I::LocalGet(index), I::LocalGet(count), I::I32GeU, I::BrIf(1)]);
				rank_of_cell(f);
				Self::emit_list(f, &increment(index));
			});
			// in order already (ascending: as it is; strictly descending: reversed, so equal keys keep their order) takes
			// n - 1 comparisons and no merging
			let (ascending, descending) = (left, right);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Const(1), I::LocalSet(ascending),
				I::I32Const(1), I::LocalSet(descending),
				I::I32Const(1), I::LocalSet(low),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(low), I::LocalGet(count), I::I32GeU, I::BrIf(1),
			]);
			Self::emit_list(f, &get(ranks, low));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::RefAsNonNull, I::LocalGet(ranks), I::LocalGet(low), I::I32Const(1), I::I32Sub, I::ArrayGet(array),
				I::RefAsNonNull,
			]);
			s.call(f, NODE_ORDER);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Const(0), I::I32LtS, I::If(BlockType::Empty),
					I::I32Const(0), I::LocalSet(ascending),
				I::Else,
					I::I32Const(0), I::LocalSet(descending),
				I::End,
				I::LocalGet(ascending), I::LocalGet(descending), I::I32Or, I::I32Eqz, I::BrIf(1),
			]);
			Self::emit_list(f, &increment(low));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Br(0),
				I::End,
				I::End,
				I::LocalGet(descending), I::If(BlockType::Empty),
					I::I32Const(0), I::LocalSet(low),
					I::LocalGet(count), I::I32Const(1), I::I32Sub, I::LocalSet(high),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(low), I::LocalGet(high), I::I32GeS, I::BrIf(1),
			]);
			for array_local in [cells, ranks] {
				Self::emit_list(f, &get(array_local, low));
				Self::emit_list(f, &[I::LocalSet(cell), I::LocalGet(array_local), I::LocalGet(low)]);
				Self::emit_list(f, &get(array_local, high));
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::ArraySet(array),
					I::LocalGet(array_local), I::LocalGet(high), I::LocalGet(cell), I::ArraySet(array),
				]);
			}
			Self::emit_list(f, &increment(low));
			Self::emit_list(
				f,
				&[I::LocalGet(high), I::I32Const(1), I::I32Sub, I::LocalSet(high), I::Br(0), I::End, I::End, I::End],
			);
			// runs of width merged pairwise into the other arrays, which then take their place
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(count), I::I32Const(1), I::LocalGet(ascending), I::LocalGet(descending), I::I32Or,
				I::Select, I::LocalSet(width),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(width), I::LocalGet(count), I::I32GeU, I::BrIf(1),
						I::I32Const(0), I::LocalSet(low),
						I::Block(BlockType::Empty),
							I::Loop(BlockType::Empty),
								I::LocalGet(low), I::LocalGet(count), I::I32GeU, I::BrIf(1),
			]);
			smaller(f, &[I::LocalGet(low), I::LocalGet(width), I::I32Add], &[I::LocalGet(count)]);
			Self::emit_list(f, &[I::LocalSet(middle), I::LocalGet(low), I::LocalSet(left)]);
			smaller(f, &[I::LocalGet(middle), I::LocalGet(width), I::I32Add], &[I::LocalGet(count)]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(high),
				I::LocalGet(middle), I::LocalSet(right),
				I::LocalGet(low), I::LocalSet(index),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(index), I::LocalGet(high), I::I32GeU, I::BrIf(1),
			]);
			// the left item unless the right one sorts before it
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(left), I::LocalGet(middle), I::I32LtU, I::If(BlockType::Result(ValType::I32)),
					I::LocalGet(right), I::LocalGet(high), I::I32GeU, I::If(BlockType::Result(ValType::I32)),
						I::I32Const(1),
					I::Else,
			]);
			Self::emit_list(f, &get(ranks, right));
			f.instruction(&I::RefAsNonNull);
			Self::emit_list(f, &get(ranks, left));
			f.instruction(&I::RefAsNonNull);
			s.call(f, NODE_ORDER);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Const(0), I::I32GeS,
				I::End,
				I::Else,
					I::I32Const(0),
				I::End,
				I::If(BlockType::Empty),
					I::LocalGet(left), I::LocalSet(from),
			]);
			Self::emit_list(f, &increment(left));
			Self::emit_list(f, &[I::Else, I::LocalGet(right), I::LocalSet(from)]);
			Self::emit_list(f, &increment(right));
			f.instruction(&I::End);
			for (to, source) in [(cells_to, cells), (ranks_to, ranks)] {
				Self::emit_list(f, &[I::LocalGet(to), I::LocalGet(index)]);
				Self::emit_list(f, &get(source, from));
				f.instruction(&I::ArraySet(array));
			}
			Self::emit_list(f, &increment(index));
			Self::emit_list(
				f,
				&[I::Br(0), I::End, I::End, I::LocalGet(high), I::LocalSet(low), I::Br(0), I::End, I::End],
			);
			for (first, second) in [(cells, cells_to), (ranks, ranks_to)] {
				Self::emit_list(f, &[I::LocalGet(first), I::LocalGet(second), I::LocalSet(first), I::LocalSet(second)]);
			}
			Self::emit_list(
				f,
				&[I::LocalGet(width), I::I32Const(1), I::I32Shl, I::LocalSet(width), I::Br(0), I::End, I::End],
			);
			// new cells from the last one back, each of its item's kind of cell
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(count), I::I32Eqz, I::BrIf(1),
						I::LocalGet(count), I::I32Const(1), I::I32Sub, I::LocalSet(count),
			]);
			Self::emit_list(f, &get(cells, count));
			Self::emit_list(f, &[I::RefAsNonNull, I::LocalSet(cell)]);
			s.emit_field(f, cell, 0);
			s.emit_field(f, cell, 1);
			Self::emit_list(
				f,
				&[I::LocalGet(sorted), I::StructNew(node_type), I::LocalSet(sorted), I::Br(0), I::End, I::End],
			);
			s.emit_list_result(f, sorted);
			f.instruction(&I::LocalSet(sorted));
			s.emit_characters_as_text(f, sorted, is_text);
		});

		if !self.should_emit_function("list_sort") {
			return;
		}
		// `sorted "hello"` is "ehllo"
		self.runtime_function("list_sort", vec![nullable], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::RefNull(HeapType::Concrete(node_type))]);
			s.call(f, LIST_SORTED_BY);
		});
	}
}
