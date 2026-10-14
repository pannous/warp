//! The runtime functions of the list backends: the typed arrays, $NodeList and its element types
use super::*;

impl WasmGcEmitter {
	/// The runtime functions of the typed-array backend, each only when the program calls it
	pub(in crate::wasm_emitter) fn emit_typed_list_runtime(&mut self) {
		for element in ElementType::ALL {
			self.emit_element_runtime(element);
		}
		self.emit_node_list_runtime();
	}

	/// The runtime functions of `$NodeList`, each only when the program calls it: as the number lists, plus the kind of the
	/// list node it stands for, and node_list_of to start from a Node list
	pub(super) fn emit_node_list_runtime(&mut self) {
		let (array, list) = self.typed_list_types(ElementType::Node);
		let list_ref = Ref(self.typed_list_ref(ElementType::Node));
		let array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let node = self.type_manager.node_type;
		let node_ref = Ref(self.node_ref(false));
		let nullable_node = Ref(self.node_ref(true));
		let length = I::StructGet { struct_type_index: list, field_index: 0 };
		let items = I::StructGet { struct_type_index: list, field_index: 1 };
		let kind = I::StructGet { struct_type_index: list, field_index: 2 };
		let new_list = I::StructNew(list);
		if self.should_emit_function(NODE_RUNTIME.at) || self.should_emit_function(NODE_RUNTIME.set) {
			self.ensure_index_out_of_range_of();
		}
		if self.should_emit_function(NODE_RUNTIME.at) {
			self.runtime_function(NODE_RUNTIME.at, vec![list_ref, ValType::I64], vec![node_ref], vec![], |s, f| {
				s.emit_list_position(f, list);
				Self::emit_list(f, &[I::ArrayGet(array), I::RefAsNonNull]);
			});
		}
		if self.should_emit_function(NODE_RUNTIME.set) {
			self.runtime_function(NODE_RUNTIME.set, vec![list_ref, ValType::I64, node_ref], vec![], vec![], |s, f| {
				s.emit_list_position(f, list);
				Self::emit_list(f, &[I::LocalGet(2), I::ArraySet(array)]);
			});
		}
		if self.should_emit_function(NODE_RUNTIME.copy) {
			self.runtime_function(NODE_RUNTIME.copy, vec![list_ref], vec![list_ref], vec![array_ref], |_, f| {
				Self::emit_list(
					f,
					&[
						I::LocalGet(0),
						length.clone(),
						I::ArrayNewDefault(array),
						I::LocalSet(1),
						I::LocalGet(1),
						I::I32Const(0),
						I::LocalGet(0),
						items.clone(),
						I::I32Const(0),
						I::LocalGet(0),
						length.clone(),
						I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
						I::LocalGet(0),
						length.clone(),
						I::LocalGet(1),
						I::RefAsNonNull,
						I::LocalGet(0),
						kind.clone(),
						new_list.clone(),
					],
				);
			});
		}
		if self.should_emit_function(NODE_RUNTIME.filled) {
			self.runtime_function(
				NODE_RUNTIME.filled,
				vec![ValType::I64, node_ref],
				vec![list_ref],
				vec![ValType::I32],
				|s, f| {
					Self::emit_list(f, &[I::LocalGet(0), I::I64Const(i32::MAX as i64), I::I64GtS]);
					s.emit_fail_if(f, "out_of_memory");
					Self::emit_list(
						f,
						&[
							I::LocalGet(0),
							I::I64Const(0),
							I::LocalGet(0),
							I::I64Const(0),
							I::I64GtS,
							I::Select,
							I::I32WrapI64,
							I::LocalTee(2),
							I::LocalGet(1),
							I::LocalGet(2),
							I::ArrayNew(array),
							I::I64Const(crate::type_kinds::SQUARE_LIST_KIND),
							new_list.clone(),
						],
					);
				},
			);
		}
		if self.should_emit_function(NODE_RUNTIME.push) {
			self.runtime_function(
				NODE_RUNTIME.push,
				vec![list_ref, node_ref],
				vec![list_ref],
				vec![array_ref],
				|_, f| {
					Self::emit_list(
						f,
						&push_in_place(
							list,
							array,
							&[I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), I::StructNew(list)],
						),
					);
				},
			);
		}
		// as_node(list): the cons cells of the items under the list's own kind (brackets); ø when empty or unassigned
		if self.should_emit_function(NODE_RUNTIME.as_node) {
			self.runtime_function(
				NODE_RUNTIME.as_node,
				vec![list_ref],
				vec![node_ref],
				vec![ValType::I32, nullable_node],
				|s, f| {
					s.emit_cells_from_last(f, list, |_, f| {
						Self::emit_list(
							f,
							&[
								I::LocalGet(0),
								kind.clone(),
								I::LocalGet(0),
								items.clone(),
								I::LocalGet(AS_NODE_POSITION),
								I::ArrayGet(array),
								I::LocalGet(AS_NODE_REST),
								I::StructNew(node),
							],
						)
					});
				},
			);
		}
		// node_list_of(node): the items of a list's cells (up to a meta entry, as a walk ends there), ø no items, any
		// other node the one item, as node_count counts it
		if self.should_emit_function(NODE_LIST_OF) {
			let (cell, out, tag) = (1, 2, 3);
			self.runtime_function(
				NODE_LIST_OF,
				vec![node_ref],
				vec![list_ref],
				vec![nullable_node, list_ref, ValType::I64],
				|s, f| {
					Self::emit_list(f, &[I::RefNull(HeapType::Concrete(list)), I::LocalSet(out)]);
					s.emit_field(f, 0, 0);
					Self::emit_list(
						f,
						&[
							I::I64Const(crate::type_kinds::KIND_MASK),
							I::I64And,
							I::LocalTee(tag),
							I::I64Const(Kind::Empty as i64),
							I::I64Eq,
							I::If(BlockType::Empty),
							I::I32Const(0),
							I::I32Const(0),
							I::ArrayNewDefault(array),
							I::I64Const(crate::type_kinds::SQUARE_LIST_KIND),
							new_list.clone(),
							I::Return,
							I::End,
							I::LocalGet(tag),
							I::I64Const(Kind::List as i64),
							I::I64Ne,
							I::LocalGet(tag),
							I::I64Const(Kind::Block as i64),
							I::I64Ne,
							I::I32And,
							I::If(BlockType::Empty),
							I::LocalGet(out),
							I::LocalGet(0),
						],
					);
					s.call(f, NODE_RUNTIME.push);
					Self::emit_list(f, &[I::Return, I::End]);
					// a list: walk its cells, the last one's value may be a single item
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(0), I::LocalSet(cell),
						I::Block(BlockType::Empty),
							I::Loop(BlockType::Empty),
								I::LocalGet(cell), I::StructGet { struct_type_index: node, field_index: 1 },
					]);
					s.call(f, super::super::super::reflection::equality::IS_META_ENTRY);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::BrIf(1),
						I::LocalGet(out), I::LocalGet(cell), I::StructGet { struct_type_index: node, field_index: 1 },
						I::RefCastNonNull(HeapType::Concrete(node)),
					]);
					s.call(f, NODE_RUNTIME.push);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalSet(out),
						I::LocalGet(cell), I::StructGet { struct_type_index: node, field_index: 2 }, I::LocalTee(cell),
						I::RefIsNull, I::BrIf(1),
						I::LocalGet(cell), I::StructGet { struct_type_index: node, field_index: 0 },
						I::I64Const(crate::type_kinds::KIND_MASK), I::I64And, I::LocalTee(tag),
						I::I64Const(Kind::List as i64), I::I64Eq, I::LocalGet(tag), I::I64Const(Kind::Block as i64),
						I::I64Eq, I::I32Or, I::BrIf(0),
						I::LocalGet(out), I::LocalGet(cell), I::RefAsNonNull,
					]);
					s.call(f, NODE_RUNTIME.push);
					Self::emit_list(f, &[I::LocalSet(out), I::End, I::End]);
					// the empty list ø and a list whose first item is a meta entry hold no items
					Self::emit_list(
						f,
						&[
							I::LocalGet(out),
							I::RefIsNull,
							I::If(BlockType::Empty),
							I::I32Const(0),
							I::I32Const(0),
							I::ArrayNewDefault(array),
							I::I64Const(crate::type_kinds::SQUARE_LIST_KIND),
							new_list.clone(),
							I::LocalSet(out),
							I::End,
							I::LocalGet(out),
						],
					);
					s.emit_field(f, 0, 0);
					Self::emit_list(f, &[I::StructSet { struct_type_index: list, field_index: 2 }, I::LocalGet(out)]);
				},
			);
		}
	}

	pub(super) fn emit_element_runtime(&mut self, element: ElementType) {
		let runtime = element.runtime();
		let value = element.val_type();
		let (array, list) = self.typed_list_types(element);
		let list_ref = Ref(self.typed_list_ref(element));
		let array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(array) });
		let node_ref = Ref(self.node_ref(false));
		let length = I::StructGet { struct_type_index: list, field_index: 0 };
		let items = I::StructGet { struct_type_index: list, field_index: 1 };
		if self.should_emit_function(runtime.at) || self.should_emit_function(runtime.set) {
			self.ensure_index_out_of_range_of();
		}
		// at(list, index) -> element: the element at the 1-based index
		if self.should_emit_function(runtime.at) {
			self.runtime_function(runtime.at, vec![list_ref, ValType::I64], vec![value], vec![], |s, f| {
				s.emit_list_position(f, list);
				f.instruction(&I::ArrayGet(array));
			});
		}
		// set(list, index, value): the element at the 1-based index becomes value
		if self.should_emit_function(runtime.set) {
			self.runtime_function(runtime.set, vec![list_ref, ValType::I64, value], vec![], vec![], |s, f| {
				s.emit_list_position(f, list);
				Self::emit_list(f, &[I::LocalGet(2), I::ArraySet(array)]);
			});
		}
		// copy(list) -> list: its own items, no spare capacity
		if self.should_emit_function(runtime.copy) {
			self.runtime_function(runtime.copy, vec![list_ref], vec![list_ref], vec![array_ref], |_, f| {
				Self::emit_list(
					f,
					&[
						I::LocalGet(0),
						length.clone(),
						I::ArrayNewDefault(array),
						I::LocalSet(1),
						I::LocalGet(1),
						I::I32Const(0),
						I::LocalGet(0),
						items.clone(),
						I::I32Const(0),
						I::LocalGet(0),
						length.clone(),
						I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
						I::LocalGet(0),
						length.clone(),
						I::LocalGet(1),
						I::RefAsNonNull,
						I::StructNew(list),
					],
				);
			});
		}
		if self.should_emit_function(runtime.concat) {
			let (first, second, first_length, second_length, joined) = (0, 1, 2, 3, 4);
			let length_or_zero = |list: u32, local: u32| {
				[
					I::LocalGet(list),
					I::RefIsNull,
					I::If(BlockType::Result(ValType::I32)),
					I::I32Const(0),
					I::Else,
					I::LocalGet(list),
					length.clone(),
					I::End,
					I::LocalSet(local),
				]
			};
			let copied = |list: u32, at: Vec<I<'static>>, count: u32| {
				[
					vec![I::LocalGet(count), I::If(BlockType::Empty), I::LocalGet(joined)],
					at,
					vec![
						I::LocalGet(list),
						items.clone(),
						I::I32Const(0),
						I::LocalGet(count),
						I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
						I::End,
					],
				]
				.concat()
			};
			self.runtime_function(
				runtime.concat,
				vec![list_ref, list_ref],
				vec![list_ref],
				vec![ValType::I32, ValType::I32, array_ref],
				|_, f| {
					Self::emit_list(f, &length_or_zero(first, first_length));
					Self::emit_list(f, &length_or_zero(second, second_length));
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(first_length), I::LocalGet(second_length), I::I32Add, I::ArrayNewDefault(array),
						I::LocalSet(joined),
					]);
					Self::emit_list(f, &copied(first, vec![I::I32Const(0)], first_length));
					Self::emit_list(f, &copied(second, vec![I::LocalGet(first_length)], second_length));
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(first_length), I::LocalGet(second_length), I::I32Add, I::LocalGet(joined),
						I::RefAsNonNull, I::StructNew(list),
					]);
				},
			);
		}
		// filled(count, zero) -> list: count times zero, empty when count is not positive
		if self.should_emit_function(runtime.filled) {
			self.runtime_function(
				runtime.filled,
				vec![ValType::I64, value],
				vec![list_ref],
				vec![ValType::I32],
				|s, f| {
					// more elements than an array index holds: out of memory, never a count wrapped to i32
					Self::emit_list(f, &[I::LocalGet(0), I::I64Const(i32::MAX as i64), I::I64GtS]);
					s.emit_fail_if(f, "out_of_memory");
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(0), I::I64Const(0), I::LocalGet(0), I::I64Const(0), I::I64GtS, I::Select,
						I::I32WrapI64, I::LocalTee(2), I::LocalGet(1), I::LocalGet(2), I::ArrayNew(array),
						I::StructNew(list),
					]);
				},
			);
		}
		// push(list, value) -> list: value appended in place, the items doubled when full; a new list for ø
		if self.should_emit_function(runtime.push) {
			self.runtime_function(runtime.push, vec![list_ref, value], vec![list_ref], vec![array_ref], |_, f| {
				Self::emit_list(f, &push_in_place(list, array, &[I::StructNew(list)]));
			});
		}
		// sum(list) -> element: left to right like the loop; an int sum gives SUM_NOT_FIXNUM when it leaves the fixnum range
		if self.should_emit_function(runtime.sum) {
			let (position, sum, item) = (1, 2, 3);
			self.runtime_function(
				runtime.sum,
				vec![list_ref],
				vec![value],
				vec![ValType::I32, value, value],
				|_, f| {
					let not_fixnum = |local| {
						[
							I::LocalGet(local),
							I::I64Const(super::super::super::numbers::big_int::FIXNUM_OFFSET),
							I::I64Add,
							I::I64Const(0),
							I::I64LtS,
							I::If(BlockType::Empty),
							I::I64Const(SUM_NOT_FIXNUM),
							I::Return,
							I::End,
						]
					};
					Self::emit_list(
						f,
						&[
							I::Block(BlockType::Empty),
							I::Loop(BlockType::Empty),
							I::LocalGet(position),
							I::LocalGet(0),
							length.clone(),
							I::I32GeU,
							I::BrIf(1),
							I::LocalGet(0),
							items.clone(),
							I::LocalGet(position),
							I::ArrayGet(array),
							I::LocalSet(item),
						],
					);
					if element == ElementType::Int {
						Self::emit_list(f, &not_fixnum(item));
						Self::emit_list(f, &[I::LocalGet(sum), I::LocalGet(item), I::I64Add, I::LocalSet(sum)]);
						Self::emit_list(f, &not_fixnum(sum));
					} else {
						Self::emit_list(f, &[I::LocalGet(sum), I::LocalGet(item), I::F64Add, I::LocalSet(sum)]);
					}
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(position), I::I32Const(1), I::I32Add, I::LocalSet(position),
						I::Br(0),
						I::End,
						I::End,
						I::LocalGet(sum),
					]);
				},
			);
		}
		// as_node_list(list) -> $NodeList: one node per element, no cons cells; ø the empty list
		if self.should_emit_function(runtime.as_node_list) {
			let (node_array, node_list) = self.typed_list_types(ElementType::Node);
			let node_list_ref = Ref(self.typed_list_ref(ElementType::Node));
			let node_array_ref = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(node_array) });
			self.runtime_function(
				runtime.as_node_list,
				vec![list_ref],
				vec![node_list_ref],
				vec![ValType::I32, node_array_ref],
				|s, f| {
					let (position, nodes) = (1, 2);
					Self::emit_list(
						f,
						&[
							I::LocalGet(0),
							I::RefIsNull,
							I::If(BlockType::Empty),
							I::I32Const(0),
							I::I32Const(0),
							I::ArrayNewDefault(node_array),
							I::I64Const(crate::type_kinds::SQUARE_LIST_KIND),
							I::StructNew(node_list),
							I::Return,
							I::End,
							I::LocalGet(0),
							length.clone(),
							I::ArrayNewDefault(node_array),
							I::LocalSet(nodes),
							I::Block(BlockType::Empty),
							I::Loop(BlockType::Empty),
							I::LocalGet(position),
							I::LocalGet(0),
							length.clone(),
							I::I32GeU,
							I::BrIf(1),
							I::LocalGet(nodes),
							I::LocalGet(position),
							I::LocalGet(0),
							items.clone(),
							I::LocalGet(position),
							I::ArrayGet(array),
						],
					);
					s.call(f, runtime.new_node);
					Self::emit_list(
						f,
						&[
							I::ArraySet(node_array),
							I::LocalGet(position),
							I::I32Const(1),
							I::I32Add,
							I::LocalSet(position),
							I::Br(0),
							I::End,
							I::End,
							I::LocalGet(0),
							length.clone(),
							I::LocalGet(nodes),
							I::RefAsNonNull,
							I::I64Const(crate::type_kinds::SQUARE_LIST_KIND),
							I::StructNew(node_list),
						],
					);
				},
			);
		}
		// as_node(list) -> ref $Node: the square list of element nodes, built from the last element; ø when empty
		if self.should_emit_function(runtime.as_node) {
			let nullable_node = Ref(self.node_ref(true));
			// a typed list variable read before its first assignment is ø, as any unassigned list variable
			self.runtime_function(
				runtime.as_node,
				vec![list_ref],
				vec![node_ref],
				vec![ValType::I32, nullable_node],
				|s, f| {
					s.emit_cells_from_last(f, list, |s, f| {
						Self::emit_list(
							f,
							&[I::LocalGet(0), items.clone(), I::LocalGet(AS_NODE_POSITION), I::ArrayGet(array)],
						);
						s.call(f, runtime.new_node);
						Self::emit_list(f, &[I::LocalGet(AS_NODE_REST), I::I64Const(SQUARE_BRACKET_INFO)]);
						s.call(f, "new_list");
					});
				},
			);
		}
	}

	/// The body of an as_node(list) runtime function (locals: position i32, rest nullable node): ø for a null or empty
	/// list, else the cells `cell` builds from the last item to the first, each in front of the cells after it (local
	/// AS_NODE_REST) from the item at AS_NODE_POSITION
	pub(super) fn emit_cells_from_last(
		&mut self,
		f: &mut Function,
		list: u32,
		cell: impl FnOnce(&mut Self, &mut Function),
	) {
		let (position, rest) = (AS_NODE_POSITION, AS_NODE_REST);
		Self::emit_list(f, &[I::LocalGet(0), I::RefIsNull, I::If(BlockType::Empty)]);
		self.call(f, "new_empty");
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::Return,
			I::End,
			I::RefNull(HeapType::Concrete(self.type_manager.node_type)), I::LocalSet(rest),
			I::LocalGet(0), I::StructGet { struct_type_index: list, field_index: 0 }, I::LocalSet(position),
			I::Block(BlockType::Empty),
				I::Loop(BlockType::Empty),
					I::LocalGet(position), I::I32Eqz, I::BrIf(1),
					I::LocalGet(position), I::I32Const(1), I::I32Sub, I::LocalSet(position),
		]);
		cell(self, f);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalSet(rest),
			I::Br(0),
			I::End,
			I::End,
			I::LocalGet(rest), I::RefIsNull, I::If(BlockType::Result(Ref(self.node_ref(false)))),
		]);
		self.call(f, "new_empty");
		Self::emit_list(f, &[I::Else, I::LocalGet(rest), I::RefAsNonNull, I::End]);
	}

	/// Push a number list variable as a $NodeList; false for any other variable
	pub(in crate::wasm_emitter) fn emit_typed_list_as_node_list(&mut self, func: &mut Function, name: &str) -> bool {
		let Some((slot, list)) = self.typed_list(&Node::Symbol(name.to_string())) else {
			return false;
		};
		if list.element == ElementType::Node {
			return false;
		}
		func.instruction(&slot.get());
		self.emit_call(func, list.element.runtime().as_node_list);
		true
	}

	/// In a runtime function (list, index, …): fail unless the 1-based index (local 1) is an integer within the list,
	/// then push its items and the 0-based i32 position
	pub(super) fn emit_list_position(&self, func: &mut Function, list: u32) {
		self.emit_require_integral(func, 1);
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(1), I::I64Const(1), I::I64LtS, I::LocalGet(1), I::LocalGet(0),
			I::StructGet { struct_type_index: list, field_index: 0 }, I::I64ExtendI32U, I::I64GtS, I::I32Or,
			I::If(BlockType::Empty),
				I::LocalGet(1), I::LocalGet(0), I::StructGet { struct_type_index: list, field_index: 0 },
				I::I64ExtendI32U,
		]);
		self.call(func, super::super::super::lists::list_ops::INDEX_OUT_OF_RANGE_OF);
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::End,
			I::LocalGet(0), I::StructGet { struct_type_index: list, field_index: 1 }, I::LocalGet(1), I::I32WrapI64,
			I::I32Const(1), I::I32Sub,
		]);
	}
}
