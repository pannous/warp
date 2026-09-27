//! List and string operation functions for WASM

use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction::I32Const;
use ValType::Ref;
use crate::type_kinds::Kind;
use crate::node::Node;

const BYTE: MemArg = MemArg { offset: 0, align: 0, memory_index: 0 };

impl WasmGcEmitter {
	/// Emit list and string operation helper functions
	pub(crate) fn emit_list_ops(&mut self) {
		let node_ref = self.node_ref(false);
		let node_ref_nullable = self.node_ref(true);
		self.emit_runtime_errors();

		// list_at(list: ref $Node, index: i64) -> i64
		// Get the numeric value of the element at index (1-based)
		// Traverses linked list: for index N, follow value pointer N-1 times, return data
		if self.should_emit_function("list_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			self.emit_list_walk(&mut func, 2);

			// Get current.data (which is a ref to the element Node, cast from anyref)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field (anyref holding ref $Node)
			});
			// Cast anyref to ref $Node
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));
			// Get the inner node's data field (which holds the i64_box)
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field of the element node
			});
			self.emit_int_from_payload(&mut func);

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_at");
			self.exports.export("list_at", ExportKind::Func, idx);
		}

		// list_node_at(list: ref $Node, index: i64) -> ref $Node
		// Get the element node at index (1-based), returns the node itself (for symbol/text lists)
		if self.should_emit_function("list_node_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			// Locals: 0=list, 1=index, 2=current (loop variable)
			let mut func = Function::new(vec![(1, Ref(node_ref_nullable))]);

			self.emit_list_walk(&mut func, 2);

			// Get current.data (which is a ref to the element Node, cast from anyref)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 1, // data field (anyref holding ref $Node)
			});
			// Cast anyref to ref $Node and return it directly
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.node_type)));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("list_node_at");
			self.exports.export("list_node_at", ExportKind::Func, idx);
		}

		// node_count(node: ref $Node) -> i64
		// Count the number of elements in a list/block by traversing the value chain
		if self.should_emit_function("node_count") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut().ty().function(vec![Ref(node_ref)], vec![ValType::I64]);
			self.functions.function(func_type);

			// Locals: 0=node, 1=count, 2=current
			let mut func = Function::new(vec![(1, ValType::I64), (1, Ref(node_ref_nullable))]);

			// count = 0
			func.instruction(&Instruction::I64Const(0));
			func.instruction(&Instruction::LocalSet(1));

			// current = node
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalSet(2));

			// Loop: while current is not null, count++ and current = current.value
			func.instruction(&Instruction::Block(BlockType::Empty));
			func.instruction(&Instruction::Loop(BlockType::Empty));

			// if current is null, break
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::RefIsNull);
			func.instruction(&Instruction::BrIf(1)); // break to outer block

			// count = count + 1
			func.instruction(&Instruction::LocalGet(1));
			func.instruction(&Instruction::I64Const(1));
			func.instruction(&Instruction::I64Add);
			func.instruction(&Instruction::LocalSet(1));

			// current = current.value (field 2)
			func.instruction(&Instruction::LocalGet(2));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 2,
			});
			func.instruction(&Instruction::LocalSet(2));

			// continue loop
			func.instruction(&Instruction::Br(0));
			func.instruction(&Instruction::End); // end loop
			func.instruction(&Instruction::End); // end block

			// return count
			func.instruction(&Instruction::LocalGet(1));

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_count");
			self.exports.export("node_count", ExportKind::Func, idx);
		}

		// string_char_at(node: ref $Node, index: i64) -> ref $Node
		// The character at index (1-based) of a Text/Symbol node as Codepoint node, decoding UTF-8
		if self.should_emit_function("string_char_at") {
			self.emit_string_char_at();
		}

		// node_index_at(node: ref $Node, index: i64) -> ref $Node
		// Runtime dispatch: for Text/Symbol call string_char_at, for List/Block call list_node_at
		if self.should_emit_function("node_index_at") {
			let func_type = self.type_manager.types().len();
			self.type_manager.types_mut()
				.ty()
				.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
			self.functions.function(func_type);

			let mut func = Function::new(vec![]);

			// Get node.kind
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 0, // kind field
			});

			// Check if kind is Text (3) or Symbol (5)
			// kind == 3 || kind == 5 means it's a string
			func.instruction(&Instruction::I64Const(3)); // Kind::Text
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			// It's a Text - call string_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "string_char_at");
			func.instruction(&Instruction::Else);
			// Check for Symbol
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet {
				struct_type_index: self.type_manager.node_type,
				field_index: 0,
			});
			func.instruction(&Instruction::I64Const(5)); // Kind::Symbol
			func.instruction(&Instruction::I64Eq);
			func.instruction(&Instruction::If(BlockType::Result(Ref(node_ref))));
			// It's a Symbol - call string_char_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "string_char_at");
			func.instruction(&Instruction::Else);
			// Otherwise it's a list - call list_node_at
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::LocalGet(1));
			self.emit_call(&mut func, "list_node_at");
			func.instruction(&Instruction::End); // end inner if
			func.instruction(&Instruction::End); // end outer if

			func.instruction(&Instruction::End);
			self.code.function(&func);
			let idx = self.register_func("node_index_at");
			self.exports.export("node_index_at", ExportKind::Func, idx);
		}

		self.emit_with_at_functions();
		self.emit_list_concat();
	}

	/// Locals: 0=node, 1=index, 2=pointer, 3=end, 4=codepoint
	fn emit_string_char_at(&mut self) {
		let node_ref = self.node_ref(false);
		let string_type = self.type_manager.string_type;
		let (pointer, end, codepoint) = (2, 3, 4);
		let func_type = self.type_manager.types().len();
		self.type_manager.types_mut()
			.ty()
			.function(vec![Ref(node_ref), ValType::I64], vec![Ref(node_ref)]);
		self.functions.function(func_type);
		let mut func = Function::new(vec![(3, ValType::I32)]);
		Self::emit_index_compare(&mut func, Instruction::I64LtS);
		self.emit_fail_if(&mut func, "index_out_of_range");
		let string_field = |func: &mut Function, field_index: u32| {
			func.instruction(&Instruction::LocalGet(0));
			func.instruction(&Instruction::StructGet { struct_type_index: self.type_manager.node_type, field_index: 1 });
			func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(string_type)));
			func.instruction(&Instruction::StructGet { struct_type_index: string_type, field_index });
		};
		string_field(&mut func, 0);
		func.instruction(&Instruction::LocalTee(pointer));
		string_field(&mut func, 1);
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::LocalSet(end));

		// skip index-1 characters: each is a lead byte followed by continuation bytes
		func.instruction(&Instruction::Block(BlockType::Empty));
		func.instruction(&Instruction::Loop(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(1));
		func.instruction(&Instruction::I64Const(1));
		func.instruction(&Instruction::I64LeS);
		func.instruction(&Instruction::BrIf(1));
		Self::emit_skip_continuation_bytes(&mut func, pointer, end, None);
		func.instruction(&Instruction::LocalGet(1));
		func.instruction(&Instruction::I64Const(1));
		func.instruction(&Instruction::I64Sub);
		func.instruction(&Instruction::LocalSet(1));
		func.instruction(&Instruction::Br(0));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);

		func.instruction(&Instruction::LocalGet(pointer));
		func.instruction(&Instruction::LocalGet(end));
		func.instruction(&Instruction::I32GeU);
		self.emit_fail_if(&mut func, "index_out_of_range");

		// decode: the lead byte keeps 7, 5, 4 or 3 payload bits, each continuation byte adds 6
		func.instruction(&Instruction::LocalGet(pointer));
		func.instruction(&Instruction::I32Load8U(BYTE));
		func.instruction(&Instruction::LocalTee(codepoint));
		func.instruction(&I32Const(0x80));
		func.instruction(&Instruction::I32GeU);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(codepoint));
		func.instruction(&I32Const(0x07));
		func.instruction(&I32Const(0x0F));
		func.instruction(&I32Const(0x1F));
		func.instruction(&Instruction::LocalGet(codepoint));
		func.instruction(&I32Const(0xE0));
		func.instruction(&Instruction::I32GeU);
		func.instruction(&Instruction::Select);
		func.instruction(&Instruction::LocalGet(codepoint));
		func.instruction(&I32Const(0xF0));
		func.instruction(&Instruction::I32GeU);
		func.instruction(&Instruction::Select);
		func.instruction(&Instruction::I32And);
		func.instruction(&Instruction::LocalSet(codepoint));
		Self::emit_skip_continuation_bytes(&mut func, pointer, end, Some(codepoint));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::LocalGet(codepoint));
		self.emit_call(&mut func, "new_codepoint");

		func.instruction(&Instruction::End);
		self.code.function(&func);
		let idx = self.register_func("string_char_at");
		self.exports.export("string_char_at", ExportKind::Func, idx);
	}

	/// pointer++ past following UTF-8 continuation bytes (10xxxxxx), shifting their 6 bits into `accumulator`
	fn emit_skip_continuation_bytes(func: &mut Function, pointer: u32, end: u32, accumulator: Option<u32>) {
		func.instruction(&Instruction::Loop(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(pointer));
		func.instruction(&I32Const(1));
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::LocalTee(pointer));
		func.instruction(&Instruction::LocalGet(end));
		func.instruction(&Instruction::I32LtU);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(pointer));
		func.instruction(&Instruction::I32Load8U(BYTE));
		func.instruction(&I32Const(0xC0));
		func.instruction(&Instruction::I32And);
		func.instruction(&I32Const(0x80));
		func.instruction(&Instruction::I32Eq);
		func.instruction(&Instruction::If(BlockType::Empty));
		if let Some(accumulator) = accumulator {
			func.instruction(&Instruction::LocalGet(accumulator));
			func.instruction(&I32Const(6));
			func.instruction(&Instruction::I32Shl);
			func.instruction(&Instruction::LocalGet(pointer));
			func.instruction(&Instruction::I32Load8U(BYTE));
			func.instruction(&I32Const(0x3F));
			func.instruction(&Instruction::I32And);
			func.instruction(&Instruction::I32Or);
			func.instruction(&Instruction::LocalSet(accumulator));
		}
		func.instruction(&Instruction::Br(2));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);
	}
}

/// Runtime errors trap inside a function of that name; eval reports the name as an error value
pub const RUNTIME_ERRORS: [&str; 3] = ["index_out_of_range", "invalid_number", "out_of_memory"];

impl WasmGcEmitter {
	fn emit_runtime_errors(&mut self) {
		for name in RUNTIME_ERRORS {
			self.runtime_function(name, vec![], vec![], vec![], |_, f| {
				f.instruction(&Instruction::Unreachable);
			});
		}
	}

	/// Unconditional runtime error; the stack after it is unreachable, so it fits any expected type
	pub(super) fn emit_runtime_error(&mut self, func: &mut Function, error: &'static str) {
		self.emit_call(func, error);
		func.instruction(&Instruction::Unreachable);
	}

	/// Trap when the i32 condition on the stack is true
	fn emit_fail_if(&self, func: &mut Function, error: &'static str) {
		func.instruction(&Instruction::If(BlockType::Empty));
		self.call(func, error);
		func.instruction(&Instruction::End);
	}

	fn emit_index_compare(func: &mut Function, compare: Instruction) {
		func.instruction(&Instruction::LocalGet(1));
		func.instruction(&Instruction::I64Const(1));
		func.instruction(&compare);
	}

	fn emit_field(&self, func: &mut Function, local: u32, field_index: u32) {
		func.instruction(&Instruction::LocalGet(local));
		func.instruction(&Instruction::StructGet { struct_type_index: self.type_manager.node_type, field_index });
	}

	/// Push field `field_index` (0 ptr, 1 len) of the $String inside text node `local`
	fn emit_text_field(&self, func: &mut Function, local: u32, field_index: u32) {
		self.emit_field(func, local, 1);
		func.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(self.type_manager.string_type)));
		func.instruction(&Instruction::StructGet { struct_type_index: self.type_manager.string_type, field_index });
	}

	/// Walk list local 0 to the element at 1-based index local 1 into `current`; trap when out of range
	fn emit_list_walk(&self, func: &mut Function, current: u32) {
		Self::emit_index_compare(func, Instruction::I64LtS);
		self.emit_fail_if(func, "index_out_of_range");
		func.instruction(&Instruction::LocalGet(0));
		func.instruction(&Instruction::LocalSet(current));
		func.instruction(&Instruction::Block(BlockType::Empty));
		func.instruction(&Instruction::Loop(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(current));
		func.instruction(&Instruction::RefIsNull);
		self.emit_fail_if(func, "index_out_of_range");
		Self::emit_index_compare(func, Instruction::I64LeS);
		func.instruction(&Instruction::BrIf(1));
		self.emit_field(func, current, 2);
		func.instruction(&Instruction::LocalSet(current));
		Self::emit_index_compare(func, Instruction::I64Sub);
		func.instruction(&Instruction::LocalSet(1));
		func.instruction(&Instruction::Br(0));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);
	}

	/// Trap unless 1-based index local 1 lies within the text of local 0
	fn emit_text_index_guard(&self, func: &mut Function) {
		Self::emit_index_compare(func, Instruction::I64LtS);
		func.instruction(&Instruction::LocalGet(1));
		self.emit_text_field(func, 0, 1);
		func.instruction(&Instruction::I64ExtendI32U);
		func.instruction(&Instruction::I64GtS);
		func.instruction(&Instruction::I32Or);
		self.emit_fail_if(func, "index_out_of_range");
	}

	fn emit_text_heap_global(&mut self) {
		self.globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(0));
		self.text_heap_global = self.next_global_idx;
		self.next_global_idx += 1;
	}

	/// Bump-allocate `length` bytes into local `address`; fresh pages come from memory.grow,
	/// so runtime texts never overlap the string table at the start of memory
	fn emit_text_allocation(&self, func: &mut Function, length: u32, address: u32) {
		const PAGE_BITS: i32 = 16;
		let heap = self.text_heap_global;
		func.instruction(&Instruction::GlobalGet(heap));
		func.instruction(&Instruction::I32Eqz);
		func.instruction(&Instruction::GlobalGet(heap));
		func.instruction(&Instruction::LocalGet(length));
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::MemorySize(0));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&Instruction::I32Shl);
		func.instruction(&Instruction::I32GtU);
		func.instruction(&Instruction::I32Or);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::LocalGet(length));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&Instruction::I32ShrU);
		func.instruction(&I32Const(1));
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::MemoryGrow(0));
		func.instruction(&Instruction::LocalTee(address));
		func.instruction(&I32Const(-1));
		func.instruction(&Instruction::I32Eq);
		self.emit_fail_if(func, "out_of_memory");
		func.instruction(&Instruction::LocalGet(address));
		func.instruction(&I32Const(PAGE_BITS));
		func.instruction(&Instruction::I32Shl);
		func.instruction(&Instruction::GlobalSet(heap));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::GlobalGet(heap));
		func.instruction(&Instruction::LocalTee(address));
		func.instruction(&Instruction::LocalGet(length));
		func.instruction(&Instruction::I32Add);
		func.instruction(&Instruction::GlobalSet(heap));
	}

	fn emit_is_text(&self, func: &mut Function) {
		for kind in [Kind::Text, Kind::Symbol] {
			self.emit_field(func, 0, 0);
			func.instruction(&Instruction::I64Const(kind as i64));
			func.instruction(&Instruction::I64Eq);
		}
		func.instruction(&Instruction::I32Or);
	}

	fn emit_forward_arguments(func: &mut Function, callee: u32) {
		for local in 0..3 {
			func.instruction(&Instruction::LocalGet(local));
		}
		func.instruction(&Instruction::Call(callee));
	}

	/// `target#index = value` leaves the assigned value (i64) on the stack; a variable target gets the updated copy
	pub(super) fn emit_index_assignment(&mut self, func: &mut Function, target: &Node, index: &Node, value: &Node) {
		let assigned = self.scratch(2);
		self.emit_node_instructions(func, target);
		self.emit_numeric_value(func, index);
		self.emit_numeric_value(func, value);
		func.instruction(&Instruction::LocalTee(assigned));
		self.emit_call(func, "node_with_at");
		let variable = match target.drop_meta() {
			Node::Symbol(name) => self.scope.lookup(name).filter(|local| local.kind.is_ref()),
			_ => None,
		};
		match variable {
			Some(local) => func.instruction(&Instruction::LocalSet(local.position)),
			None => func.instruction(&Instruction::Drop),
		};
		func.instruction(&Instruction::LocalGet(assigned));
	}

	/// Arithmetic without an implicit conversion: `"5"+3` and `[1 2]*2` are type errors, `list + list` concatenates.
	/// Returns true when it emitted the whole expression (a Node, or a trap after a recorded type error).
	pub(super) fn emit_typed_arithmetic(&mut self, func: &mut Function, left: &Node, op: &crate::operators::Op, right: &Node) -> bool {
		match self.arithmetic_type(left, op, right) {
			Kind::List => {
				self.emit_node_instructions(func, left);
				self.emit_node_instructions(func, right);
				self.emit_call(func, "list_concat");
				func.instruction(&Instruction::RefAsNonNull);
				true
			}
			kind => self.emit_arithmetic_type_error(func, left, op, right, kind),
		}
	}

	/// In a numeric context any collection or text operand is a type error
	pub(super) fn emit_arithmetic_type_error(&mut self, func: &mut Function, left: &Node, op: &crate::operators::Op, right: &Node, kind: Kind) -> bool {
		if !matches!(kind, Kind::Error | Kind::List) {
			return false;
		}
		let (left_kind, right_kind) = (self.get_type(left), self.get_type(right));
		let fix = if left_kind == Kind::List || right_kind == Kind::List {
			"lists only concatenate with lists (+), element-wise arithmetic needs an explicit map"
		} else {
			"no implicit conversion, convert explicitly, e.g. int(\"5\") + 3"
		};
		self.emit_type_error(func, format!("type error: {left_kind} {op} {right_kind}: {fix}"));
		true
	}

	pub(super) fn arithmetic_type(&self, left: &Node, op: &crate::operators::Op, right: &Node) -> Kind {
		crate::analyzer::arithmetic_kind(self.get_type(left), op, self.get_type(right))
	}

	/// ø (an Empty node) in local `list` becomes null, the end of a cons list
	fn emit_empty_as_null(&self, func: &mut Function, list: u32) {
		func.instruction(&Instruction::LocalGet(list));
		func.instruction(&Instruction::RefIsNull);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::Else);
		self.emit_field(func, list, 0);
		func.instruction(&Instruction::I64Const(Kind::Empty as i64));
		func.instruction(&Instruction::I64Eq);
		func.instruction(&Instruction::If(BlockType::Empty));
		func.instruction(&Instruction::RefNull(HeapType::Concrete(self.type_manager.node_type)));
		func.instruction(&Instruction::LocalSet(list));
		func.instruction(&Instruction::End);
		func.instruction(&Instruction::End);
	}

	/// list_concat(a, b): copies the cells of a in front of b, which is shared
	fn emit_list_concat(&mut self) {
		if !self.should_emit_function("list_concat") {
			return;
		}
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let list_concat = self.ctx.func_registry.import_count() + self.ctx.func_registry.code_count();
		let params = vec![node_ref_nullable, node_ref_nullable];
		self.runtime_function("list_concat", params, vec![node_ref_nullable], vec![], |s, f| {
			for list in 0..2 {
				s.emit_empty_as_null(f, list);
			}
			f.instruction(&Instruction::LocalGet(0));
			f.instruction(&Instruction::RefIsNull);
			f.instruction(&Instruction::If(BlockType::Result(node_ref_nullable)));
			f.instruction(&Instruction::LocalGet(1));
			f.instruction(&Instruction::Else);
			s.emit_field(f, 0, 0);
			s.emit_field(f, 0, 1);
			s.emit_field(f, 0, 2);
			f.instruction(&Instruction::LocalGet(1));
			f.instruction(&Instruction::Call(list_concat));
			f.instruction(&Instruction::StructNew(node_type));
			f.instruction(&Instruction::End);
		});
		assert_eq!(self.func_index("list_concat"), list_concat, "recursive call index");
	}

	/// `x#i += v` → `x#i = x#i + v`, leaving the assigned value (i64) on the stack
	pub(super) fn emit_compound_index_assignment(&mut self, func: &mut Function, left: &Node, op: &crate::operators::Op, right: &Node) -> bool {
		let Node::Key(target, crate::operators::Op::Hash, index) = left.drop_meta() else {
			return false;
		};
		let updated = Node::Key(Box::new(left.clone()), op.base_op(), Box::new(right.clone()));
		self.emit_index_assignment(func, target, index, &updated);
		true
	}

	/// Index assignment builds a new value instead of mutating a shared one (value semantics):
	/// `y#i=v` stores `node_with_at(y, i, v)` back into `y`, so aliases and deduplicated literals never change.
	fn emit_with_at_functions(&mut self) {
		if !self.should_emit_function("node_with_at") {
			return;
		}
		self.emit_text_heap_global();
		let node_ref = Ref(self.node_ref(false));
		let node_ref_nullable = Ref(self.node_ref(true));
		let node_type = self.type_manager.node_type;
		let string_type = self.type_manager.string_type;
		let next_index = |s: &Self| s.ctx.func_registry.import_count() + s.ctx.func_registry.code_count();

		// list_with_at(list, index, value): copies the cells up to index, shares the rest
		let list_with_at = next_index(self);
		let params = vec![node_ref_nullable, ValType::I64, ValType::I64];
		self.runtime_function("list_with_at", params, vec![node_ref], vec![], |s, f| {
			f.instruction(&Instruction::LocalGet(0));
			f.instruction(&Instruction::RefIsNull);
			s.emit_fail_if(f, "index_out_of_range");
			Self::emit_index_compare(f, Instruction::I64LtS);
			s.emit_fail_if(f, "index_out_of_range");
			s.emit_field(f, 0, 0);
			Self::emit_index_compare(f, Instruction::I64Eq);
			f.instruction(&Instruction::If(BlockType::Result(Ref(RefType::ANYREF))));
			f.instruction(&Instruction::LocalGet(2));
			s.call(f, "new_int");
			f.instruction(&Instruction::Else);
			s.emit_field(f, 0, 1);
			f.instruction(&Instruction::End);
			Self::emit_index_compare(f, Instruction::I64GtS);
			f.instruction(&Instruction::If(BlockType::Result(node_ref_nullable)));
			s.emit_field(f, 0, 2);
			Self::emit_index_compare(f, Instruction::I64Sub);
			f.instruction(&Instruction::LocalGet(2));
			f.instruction(&Instruction::Call(list_with_at));
			f.instruction(&Instruction::Else);
			s.emit_field(f, 0, 2);
			f.instruction(&Instruction::End);
			f.instruction(&Instruction::StructNew(node_type));
		});
		assert_eq!(self.func_index("list_with_at"), list_with_at, "recursive call index");

		// text_with_char_at(text, index, value): a fresh copy of the bytes with one byte replaced
		let text_with_char_at = next_index(self);
		let params = vec![node_ref, ValType::I64, ValType::I64];
		self.runtime_function("text_with_char_at", params.clone(), vec![node_ref], vec![ValType::I32, ValType::I32], |s, f| {
			let (length, copy) = (3, 4);
			s.emit_text_index_guard(f);
			s.emit_text_field(f, 0, 1);
			f.instruction(&Instruction::LocalSet(length));
			s.emit_text_allocation(f, length, copy);
			f.instruction(&Instruction::LocalGet(copy));
			s.emit_text_field(f, 0, 0);
			f.instruction(&Instruction::LocalGet(length));
			f.instruction(&Instruction::MemoryCopy { src_mem: 0, dst_mem: 0 });
			f.instruction(&Instruction::LocalGet(copy));
			f.instruction(&Instruction::LocalGet(1));
			f.instruction(&Instruction::I32WrapI64);
			f.instruction(&Instruction::I32Add);
			f.instruction(&I32Const(1));
			f.instruction(&Instruction::I32Sub);
			f.instruction(&Instruction::LocalGet(2));
			f.instruction(&Instruction::I32WrapI64);
			f.instruction(&Instruction::I32Store8(MemArg { offset: 0, align: 0, memory_index: 0 }));
			s.emit_field(f, 0, 0);
			f.instruction(&Instruction::LocalGet(copy));
			f.instruction(&Instruction::LocalGet(length));
			f.instruction(&Instruction::StructNew(string_type));
			f.instruction(&Instruction::RefNull(HeapType::Concrete(node_type)));
			f.instruction(&Instruction::StructNew(node_type));
		});

		// node_with_at(node, index, value): texts and symbols by byte, everything else as list
		self.runtime_function("node_with_at", params, vec![node_ref], vec![], |s, f| {
			s.emit_is_text(f);
			f.instruction(&Instruction::If(BlockType::Result(node_ref)));
			Self::emit_forward_arguments(f, text_with_char_at);
			f.instruction(&Instruction::Else);
			Self::emit_forward_arguments(f, list_with_at);
			f.instruction(&Instruction::End);
		});
	}
}

