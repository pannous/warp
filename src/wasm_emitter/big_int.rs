//! Unbounded Int: an i64 fast path that promotes to BigInt on overflow.
//!
//! An Int is one i64 everywhere (locals, params, returns, globals). Values in the fixnum
//! range `[FIXNUM_MIN, FIXNUM_MAX] = [-(2^62-1), 2^62]` stand for themselves. Every other
//! i64 is a handle `i64::MIN + index` into the `$BigInts` heap of normalized `$BigInt`s.
//! Invariant: a value in fixnum range is never a handle, so a fixnum equals only itself.
//!
//! Fast path: plain i64 instructions guarded by a range test, `(x + FIXNUM_OFFSET) >= 0`.
//! Operands proven to be fixnums (see `int_range`) skip their test, fully proven
//! expressions compile to bare i64 instructions. Only the overflow branch calls the
//! runtime below, and only the runtime allocates.
//!
//! `$BigInt` is sign-magnitude: `negative` flag plus little-endian base 2^32 limbs with no
//! leading zero limb (zero has no limbs and is never negative).
//! The runtime is emitted only when the program does integer arithmetic ("int_runtime").
//! Opt-out: `expr as i64` computes `expr` with wrapping machine arithmetic.

use super::WasmGcEmitter;
use crate::extensions::numbers::Number;
use crate::node::Node;
use crate::operators::Op;
use num_bigint::{BigInt, Sign};
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

pub const INT_RUNTIME: &str = "int_runtime";
pub const FIXNUM_OFFSET: i64 = (1 << 62) - 1;
pub const FIXNUM_MIN: i128 = -((1 << 62) - 1);
pub const FIXNUM_MAX: i128 = 1 << 62;
const HANDLE_BASE: i64 = i64::MIN;
/// Operands in [-2^31, 2^31) multiply without leaving the fixnum range
const MUL_FAST_BIAS: i64 = 1 << 31;
const INITIAL_HEAP_SIZE: i32 = 16;
const LIMB_BASE: f64 = 4294967296.0;

/// Proven value range of an integer expression; None when unknown
pub type IntRange = Option<(i128, i128)>;

pub fn is_fixnum_range(range: IntRange) -> bool {
	matches!(range, Some((low, high)) if low >= FIXNUM_MIN && high <= FIXNUM_MAX)
}

fn fits(range: IntRange, low: i128, high: i128) -> bool {
	matches!(range, Some((l, h)) if l >= low && h <= high)
}

pub fn is_fixnum(n: i64) -> bool {
	is_fixnum_range(Some((n as i128, n as i128)))
}

fn combine(left: IntRange, right: IntRange, op: impl Fn(i128, i128) -> Option<i128>) -> IntRange {
	let ((a, b), (c, d)) = (left?, right?);
	let corners = [op(a, c)?, op(a, d)?, op(b, c)?, op(b, d)?];
	Some((*corners.iter().min()?, *corners.iter().max()?))
}

/// Scratch locals every function reserves for the inline checks (a, b, result)
pub const INT_SCRATCH_LOCALS: u32 = 3;

impl WasmGcEmitter {
	pub(crate) fn int_runtime(&self) -> bool {
		self.should_emit_function(INT_RUNTIME)
	}

	fn limbs_ref(&self) -> ValType {
		Ref(RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.limbs_type) })
	}

	fn big_ref(&self) -> ValType {
		Ref(RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.big_int_type) })
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Range analysis: which expressions are proven to stay fixnums
	// ═══════════════════════════════════════════════════════════════════════

	pub(crate) fn int_range(&self, node: &Node) -> IntRange {
		let node = node.drop_meta();
		match node {
			Node::Number(Number::Int(n)) => Some((*n as i128, *n as i128)),
			Node::True => Some((1, 1)),
			Node::False => Some((0, 0)),
			Node::Char(_) => Some((0, char::MAX as i128)),
			Node::Key(left, op, right) if matches!(left.drop_meta(), Node::Empty) && (op.is_prefix() || *op == Op::Hash) => match op {
				Op::Neg => self.int_range(right).map(|(low, high)| (-high, -low)),
				Op::Not => Some((0, 1)),
				Op::Hash => Some((0, i32::MAX as i128)),
				_ => None,
			},
			Node::Key(left, op, right) => {
				let (l, r) = (self.int_range(left), self.int_range(right));
				match op {
					Op::Add => combine(l, r, i128::checked_add),
					Op::Sub => combine(l, r, i128::checked_sub),
					Op::Mul => combine(l, r, i128::checked_mul),
					Op::Mod => r.map(|(low, high)| {
						let bound = low.abs().max(high.abs()) - 1;
						(-bound, bound)
					}),
					op if op.is_comparison() => Some((0, 1)),
					_ => None,
				}
			}
			_ => None,
		}
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Inline fast paths used by the expression emitter
	// ═══════════════════════════════════════════════════════════════════════

	fn scratch(&self, index: u32) -> u32 {
		self.int_scratch + index
	}

	/// Push i32 1 when every listed local holds a fixnum: `((x + OFFSET) | ...) >= 0`
	fn emit_fixnum_test(&self, func: &mut Function, locals: &[u32]) {
		if locals.is_empty() {
			func.instruction(&I::I32Const(1));
			return;
		}
		for (i, local) in locals.iter().enumerate() {
			func.instruction(&I::LocalGet(*local));
			func.instruction(&I::I64Const(FIXNUM_OFFSET));
			func.instruction(&I::I64Add);
			if i > 0 {
				func.instruction(&I::I64Or);
			}
		}
		func.instruction(&I::I64Const(0));
		func.instruction(&I::I64GeS);
	}

	fn unproven(&self, candidates: &[(u32, IntRange)]) -> Vec<u32> {
		candidates.iter().filter(|(_, range)| !is_fixnum_range(*range)).map(|(local, _)| *local).collect()
	}

	/// Stack [a, b] → [a op b] for + - * / % ^ xor on Ints
	pub(crate) fn emit_int_op(&mut self, func: &mut Function, op: &Op, left: IntRange, right: IntRange) {
		if !self.int_runtime() {
			return self.emit_machine_int_op(func, op);
		}
		if self.wrapping_ints {
			return self.emit_wrapping_int_op(func, op, left, right);
		}
		let result = match op {
			Op::Add => combine(left, right, i128::checked_add),
			Op::Sub => combine(left, right, i128::checked_sub),
			Op::Mul => combine(left, right, i128::checked_mul),
			_ => None,
		};
		let proven = is_fixnum_range(left) && is_fixnum_range(right);
		if proven && is_fixnum_range(result) && matches!(op, Op::Add | Op::Sub | Op::Mul) {
			return self.emit_machine_int_op(func, op);
		}
		let (a, b, r) = (self.scratch(0), self.scratch(1), self.scratch(2));
		func.instruction(&I::LocalSet(b));
		func.instruction(&I::LocalSet(a));
		match op {
			Op::Add | Op::Sub => {
				func.instruction(&I::LocalGet(a));
				func.instruction(&I::LocalGet(b));
				func.instruction(&if *op == Op::Add { I::I64Add } else { I::I64Sub });
				func.instruction(&I::LocalSet(r));
				let unproven = self.unproven(&[(r, result), (a, left), (b, right)]);
				self.emit_fixnum_test(func, &unproven);
				self.emit_fast_or_slow(func, &[I::LocalGet(r)], if *op == Op::Add { "int_add_slow" } else { "int_sub_slow" });
			}
			Op::Mul => {
				let small = |range| fits(range, -MUL_FAST_BIAS as i128, MUL_FAST_BIAS as i128 - 1);
				let unproven: Vec<u32> = [(a, left), (b, right)].iter().filter(|(_, range)| !small(*range)).map(|(l, _)| *l).collect();
				Self::emit_mul_fast_test(func, &unproven);
				self.emit_fast_or_slow(func, &[I::LocalGet(a), I::LocalGet(b), I::I64Mul], "int_mul_slow");
			}
			Op::Div | Op::Mod => {
				let unproven = self.unproven(&[(a, left), (b, right)]);
				self.emit_fixnum_test(func, &unproven);
				if *op == Op::Mod {
					self.emit_fast_or_slow(func, &[I::LocalGet(a), I::LocalGet(b), I::I64RemS], "int_rem_slow");
				} else {
					// fixnum / -1 can leave the range: 2^62 / -1
					func.instruction(&I::If(BlockType::Result(ValType::I64)));
					Self::emit_list(func, &[I::LocalGet(a), I::LocalGet(b), I::I64DivS]);
					self.emit_int_from_machine(func);
					func.instruction(&I::Else);
					self.emit_scratch_call(func, "int_quot_slow");
					func.instruction(&I::End);
				}
			}
			Op::Pow => self.emit_scratch_call(func, "int_pow"),
			Op::Xor => self.emit_scratch_call(func, "int_xor"),
			_ => unreachable!("not an Int operator: {:?}", op),
		}
	}

	fn emit_scratch_call(&mut self, func: &mut Function, name: &'static str) {
		func.instruction(&I::LocalGet(self.scratch(0)));
		func.instruction(&I::LocalGet(self.scratch(1)));
		self.emit_call(func, name);
	}

	/// i32 on stack: fast instructions if true, else `slow(a, b)`
	fn emit_fast_or_slow(&mut self, func: &mut Function, fast: &[Instruction], slow: &'static str) {
		func.instruction(&I::If(BlockType::Result(ValType::I64)));
		for instruction in fast {
			func.instruction(instruction);
		}
		func.instruction(&I::Else);
		self.emit_scratch_call(func, slow);
		func.instruction(&I::End);
	}

	/// Push i32 1 when all locals are in [-2^31, 2^31): `((x + 2^31) | ...) >>u 32 == 0`
	fn emit_mul_fast_test(func: &mut Function, locals: &[u32]) {
		if locals.is_empty() {
			func.instruction(&I::I32Const(1));
			return;
		}
		for (i, local) in locals.iter().enumerate() {
			func.instruction(&I::LocalGet(*local));
			func.instruction(&I::I64Const(MUL_FAST_BIAS));
			func.instruction(&I::I64Add);
			if i > 0 {
				func.instruction(&I::I64Or);
			}
		}
		func.instruction(&I::I64Const(32));
		func.instruction(&I::I64ShrU);
		func.instruction(&I::I64Eqz);
	}

	fn emit_machine_int_op(&mut self, func: &mut Function, op: &Op) {
		match op {
			Op::Add => func.instruction(&I::I64Add),
			Op::Sub => func.instruction(&I::I64Sub),
			Op::Mul => func.instruction(&I::I64Mul),
			Op::Div => func.instruction(&I::I64DivS),
			Op::Mod => func.instruction(&I::I64RemS),
			Op::Xor => func.instruction(&I::I64Xor),
			Op::Pow => {
				self.emit_call(func, "i64_pow");
				func
			}
			_ => unreachable!("not an Int operator: {:?}", op),
		};
	}

	/// `as i64` region: operands reduced mod 2^64, machine op, result re-entered as Int
	fn emit_wrapping_int_op(&mut self, func: &mut Function, op: &Op, left: IntRange, right: IntRange) {
		let b = self.scratch(1);
		func.instruction(&I::LocalSet(b));
		if !is_fixnum_range(left) {
			self.emit_int_to_machine(func);
		}
		func.instruction(&I::LocalGet(b));
		if !is_fixnum_range(right) {
			self.emit_int_to_machine(func);
		}
		self.emit_machine_int_op(func, op);
		self.emit_int_from_machine(func);
	}

	/// `value as i64`: machine arithmetic inside, the result reduced mod 2^64
	pub(crate) fn emit_wrapping_int(&mut self, func: &mut Function, value: &Node) {
		let outer = std::mem::replace(&mut self.wrapping_ints, true);
		self.emit_numeric_value(func, value);
		self.wrapping_ints = outer;
		if self.int_runtime() && !is_fixnum_range(self.int_range(value)) {
			self.emit_int_to_machine(func);
			self.emit_int_from_machine(func);
		}
	}

	/// Int → i64 reduced mod 2^64 (identity on fixnums)
	pub(crate) fn emit_int_to_machine(&mut self, func: &mut Function) {
		let a = self.scratch(0);
		func.instruction(&I::LocalSet(a));
		self.emit_fixnum_test(func, &[a]);
		self.emit_fast_or_unary_slow(func, a, &[I::LocalGet(a)], "int_to_i64_wrap", ValType::I64);
	}

	/// Raw machine i64 → Int (promotes values outside the fixnum range)
	pub(crate) fn emit_int_from_machine(&mut self, func: &mut Function) {
		if !self.int_runtime() {
			return;
		}
		let r = self.scratch(2);
		func.instruction(&I::LocalSet(r));
		self.emit_fixnum_test(func, &[r]);
		self.emit_fast_or_unary_slow(func, r, &[I::LocalGet(r)], "int_from_i64", ValType::I64);
	}

	fn emit_fast_or_unary_slow(&mut self, func: &mut Function, local: u32, fast: &[Instruction], slow: &'static str, result: ValType) {
		func.instruction(&I::If(BlockType::Result(result)));
		for instruction in fast {
			func.instruction(instruction);
		}
		func.instruction(&I::Else);
		func.instruction(&I::LocalGet(local));
		self.emit_call(func, slow);
		func.instruction(&I::End);
	}

	/// Stack [x] → [-x]
	pub(crate) fn emit_int_neg(&mut self, func: &mut Function, range: IntRange) {
		let negated = range.map(|(low, high)| (-high, -low));
		if !self.int_runtime() || (is_fixnum_range(range) && is_fixnum_range(negated)) {
			func.instruction(&I::I64Const(-1));
			func.instruction(&I::I64Mul);
			return;
		}
		if self.wrapping_ints {
			self.emit_int_to_machine(func);
			func.instruction(&I::I64Const(-1));
			func.instruction(&I::I64Mul);
			return self.emit_int_from_machine(func);
		}
		let (a, r) = (self.scratch(0), self.scratch(2));
		func.instruction(&I::LocalTee(a));
		func.instruction(&I::I64Const(-1));
		func.instruction(&I::I64Mul);
		func.instruction(&I::LocalSet(r));
		let unproven = self.unproven(&[(r, None), (a, range)]);
		self.emit_fixnum_test(func, &unproven);
		self.emit_fast_or_unary_slow(func, a, &[I::LocalGet(r)], "int_neg_slow", ValType::I64);
	}

	/// Stack [x] → [|x|]; |fixnum| is always a fixnum
	pub(crate) fn emit_int_abs(&mut self, func: &mut Function, range: IntRange) {
		let a = self.scratch(0);
		func.instruction(&I::LocalSet(a));
		let fast = [
			I::I64Const(0), I::LocalGet(a), I::I64Sub, I::LocalGet(a), I::LocalGet(a),
			I::I64Const(0), I::I64LtS, I::Select,
		];
		if !self.int_runtime() || is_fixnum_range(range) {
			for instruction in &fast {
				func.instruction(instruction);
			}
			return;
		}
		self.emit_fixnum_test(func, &[a]);
		self.emit_fast_or_unary_slow(func, a, &fast, "int_abs_slow", ValType::I64);
	}

	/// Stack [a, b] → [i32 a op b] for comparisons
	pub(crate) fn emit_int_compare(&mut self, func: &mut Function, op: &Op, left: IntRange, right: IntRange) {
		let machine = Self::machine_compare(op);
		let equality_is_exact = matches!(op, Op::Eq | Op::Ne) && (is_fixnum_range(left) || is_fixnum_range(right));
		if !self.int_runtime() || equality_is_exact || (is_fixnum_range(left) && is_fixnum_range(right)) {
			func.instruction(&machine);
			return;
		}
		let (a, b) = (self.scratch(0), self.scratch(1));
		func.instruction(&I::LocalSet(b));
		func.instruction(&I::LocalSet(a));
		let unproven = self.unproven(&[(a, left), (b, right)]);
		self.emit_fixnum_test(func, &unproven);
		func.instruction(&I::If(BlockType::Result(ValType::I32)));
		func.instruction(&I::LocalGet(a));
		func.instruction(&I::LocalGet(b));
		func.instruction(&machine);
		func.instruction(&I::Else);
		self.emit_scratch_call(func, "int_cmp");
		func.instruction(&I::I32Const(0));
		func.instruction(&Self::i32_compare(op));
		func.instruction(&I::End);
	}

	fn machine_compare(op: &Op) -> Instruction<'static> {
		match op {
			Op::Eq => I::I64Eq,
			Op::Ne => I::I64Ne,
			Op::Lt => I::I64LtS,
			Op::Gt => I::I64GtS,
			Op::Le => I::I64LeS,
			Op::Ge => I::I64GeS,
			_ => unreachable!("Not a comparison op: {:?}", op),
		}
	}

	fn i32_compare(op: &Op) -> Instruction<'static> {
		match op {
			Op::Eq => I::I32Eq,
			Op::Ne => I::I32Ne,
			Op::Lt => I::I32LtS,
			Op::Gt => I::I32GtS,
			Op::Le => I::I32LeS,
			Op::Ge => I::I32GeS,
			_ => unreachable!("Not a comparison op: {:?}", op),
		}
	}

	/// Stack [Int] → [f64]
	pub(crate) fn emit_int_to_f64(&mut self, func: &mut Function, range: IntRange) {
		if !self.int_runtime() || is_fixnum_range(range) {
			func.instruction(&I::F64ConvertI64S);
			return;
		}
		let a = self.scratch(0);
		func.instruction(&I::LocalSet(a));
		self.emit_fixnum_test(func, &[a]);
		self.emit_fast_or_unary_slow(func, a, &[I::LocalGet(a), I::F64ConvertI64S], "int_to_f64", ValType::F64);
	}

	/// Integer literal of any size as an Int
	pub(crate) fn emit_int_literal(&mut self, func: &mut Function, number: &BigInt) {
		if let Some(n) = i64::try_from(number).ok().filter(|n| is_fixnum(*n)) {
			func.instruction(&I::I64Const(n));
			return;
		}
		if !self.int_runtime() {
			// Only reachable for i64 literals in programs without arithmetic: no handles exist
			func.instruction(&I::I64Const(i64::try_from(number).expect("big literal needs the Int runtime")));
			return;
		}
		let (sign, limbs) = number.to_u32_digits();
		func.instruction(&I::I32Const((sign == Sign::Minus) as i32));
		for limb in &limbs {
			func.instruction(&I::I32Const(*limb as i32));
		}
		func.instruction(&I::ArrayNewFixed { array_type_index: self.type_manager.limbs_type, array_size: limbs.len() as u32 });
		func.instruction(&I::StructNew(self.type_manager.big_int_type));
		self.emit_call(func, "int_box");
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Node boundary: Int payload is $i64box for fixnums, the $BigInt otherwise
	// ═══════════════════════════════════════════════════════════════════════

	/// Globals behind handles; call before any function that touches them
	pub(crate) fn emit_int_heap_globals(&mut self) {
		if !self.int_runtime() {
			return;
		}
		let heap = RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.big_heap_type) };
		self.globals.global(
			GlobalType { val_type: Ref(heap), mutable: true, shared: false },
			&ConstExpr::ref_null(HeapType::Concrete(self.type_manager.big_heap_type)),
		);
		self.int_heap_global = self.next_global_idx;
		self.globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(0));
		self.int_count_global = self.next_global_idx + 1;
		let limbs = RefType { nullable: true, heap_type: HeapType::Concrete(self.type_manager.limbs_type) };
		self.globals.global(
			GlobalType { val_type: Ref(limbs), mutable: true, shared: false },
			&ConstExpr::ref_null(HeapType::Concrete(self.type_manager.limbs_type)),
		);
		self.int_remainder_global = self.next_global_idx + 2;
		self.next_global_idx += 3;
	}

	/// Push the anyref payload for the Int in `local`
	pub(crate) fn emit_int_payload(&self, func: &mut Function, local: u32) {
		if !self.int_runtime() {
			func.instruction(&I::LocalGet(local));
			func.instruction(&I::StructNew(self.type_manager.i64_box_type));
			return;
		}
		self.emit_fixnum_test(func, &[local]);
		func.instruction(&I::If(BlockType::Result(Ref(RefType { nullable: true, heap_type: crate::type_kinds::any_heap_type() }))));
		func.instruction(&I::LocalGet(local));
		func.instruction(&I::StructNew(self.type_manager.i64_box_type));
		func.instruction(&I::Else);
		self.emit_heap_get(func, local);
		func.instruction(&I::End);
	}

	/// new_int(Int) -> Int node whose payload is $i64box or the $BigInt behind a handle
	pub(crate) fn emit_new_unbounded_int(&mut self) {
		if !self.should_emit_function("new_int") {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let node_type = self.type_manager.node_type;
		self.runtime_function("new_int", vec![ValType::I64], vec![node_ref], vec![], |s, f| {
			s.emit_kind(f, crate::type_kinds::Kind::Int);
			s.emit_int_payload(f, 0);
			f.instruction(&I::RefNull(HeapType::Concrete(node_type)));
			f.instruction(&I::StructNew(node_type));
		});
		let idx = self.func_index("new_int");
		self.exports.export("new_int", ExportKind::Func, idx);
	}

	fn emit_heap_get(&self, func: &mut Function, handle_local: u32) {
		func.instruction(&I::GlobalGet(self.int_heap_global));
		func.instruction(&I::LocalGet(handle_local));
		func.instruction(&I::I64Const(HANDLE_BASE));
		func.instruction(&I::I64Sub);
		func.instruction(&I::I32WrapI64);
		func.instruction(&I::ArrayGet(self.type_manager.big_heap_type));
	}

	/// Stack [anyref payload of an Int node] → [Int]
	pub(crate) fn emit_int_from_payload(&mut self, func: &mut Function) {
		if self.int_runtime() {
			self.emit_call(func, "int_from_payload");
			return;
		}
		func.instruction(&I::RefCastNonNull(HeapType::Concrete(self.type_manager.i64_box_type)));
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.i64_box_type, field_index: 0 });
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Runtime functions (emitted once, only when the program needs them)
	// ═══════════════════════════════════════════════════════════════════════

	fn runtime_function(
		&mut self,
		name: &'static str,
		params: Vec<ValType>,
		results: Vec<ValType>,
		locals: Vec<ValType>,
		body: impl FnOnce(&Self, &mut Function),
	) {
		let func_type = self.type_manager.types().len();
		self.type_manager.types_mut().ty().function(params, results);
		self.functions.function(func_type);
		let mut func = Function::new(locals.into_iter().map(|t| (1, t)).collect::<Vec<_>>());
		body(self, &mut func);
		func.instruction(&I::End);
		self.code.function(&func);
		self.register_func(name);
	}

	fn call(&self, func: &mut Function, name: &str) {
		func.instruction(&I::Call(self.func_index(name)));
	}

	fn limbs_get(&self, func: &mut Function) {
		func.instruction(&I::ArrayGet(self.type_manager.limbs_type));
		func.instruction(&I::I64ExtendI32U);
	}

	fn limbs_set(&self, func: &mut Function) {
		func.instruction(&I::ArraySet(self.type_manager.limbs_type));
	}

	/// Push limb `index_local` of `limbs_local` as u64, 0 past the end
	fn limb_or_zero(&self, func: &mut Function, limbs_local: u32, index: Instruction) {
		func.instruction(&index);
		func.instruction(&I::LocalGet(limbs_local));
		func.instruction(&I::ArrayLen);
		func.instruction(&I::I32LtU);
		func.instruction(&I::If(BlockType::Result(ValType::I64)));
		func.instruction(&I::LocalGet(limbs_local));
		func.instruction(&index);
		self.limbs_get(func);
		func.instruction(&I::Else);
		func.instruction(&I::I64Const(0));
		func.instruction(&I::End);
	}

	fn big_field(&self, func: &mut Function, local: u32, field_index: u32) {
		func.instruction(&I::LocalGet(local));
		func.instruction(&I::StructGet { struct_type_index: self.type_manager.big_int_type, field_index });
	}

	fn negative(&self, func: &mut Function, local: u32) {
		self.big_field(func, local, 0);
	}

	fn magnitude(&self, func: &mut Function, local: u32) {
		self.big_field(func, local, 1);
	}

	/// `local += delta` for an i32 local
	fn increment(func: &mut Function, local: u32, delta: i32) {
		func.instruction(&I::LocalGet(local));
		func.instruction(&I::I32Const(delta));
		func.instruction(&I::I32Add);
		func.instruction(&I::LocalSet(local));
	}

	/// `loop { if counter >= bound break; body; counter++ }` with i32 locals
	fn count_up(func: &mut Function, counter: u32, bound: u32, body: impl FnOnce(&mut Function)) {
		func.instruction(&I::Block(BlockType::Empty));
		func.instruction(&I::Loop(BlockType::Empty));
		func.instruction(&I::LocalGet(counter));
		func.instruction(&I::LocalGet(bound));
		func.instruction(&I::I32GeU);
		func.instruction(&I::BrIf(1));
		body(func);
		Self::increment(func, counter, 1);
		func.instruction(&I::Br(0));
		func.instruction(&I::End);
		func.instruction(&I::End);
	}

	/// `loop { if counter == 0 break; counter--; body }` with an i32 local
	fn count_down(func: &mut Function, counter: u32, body: impl FnOnce(&mut Function)) {
		func.instruction(&I::Block(BlockType::Empty));
		func.instruction(&I::Loop(BlockType::Empty));
		func.instruction(&I::LocalGet(counter));
		func.instruction(&I::I32Eqz);
		func.instruction(&I::BrIf(1));
		Self::increment(func, counter, -1);
		body(func);
		func.instruction(&I::Br(0));
		func.instruction(&I::End);
		func.instruction(&I::End);
	}

	fn emit_list(func: &mut Function, instructions: &[Instruction]) {
		for instruction in instructions {
			func.instruction(instruction);
		}
	}

	pub(crate) fn emit_int_runtime(&mut self) {
		if !self.int_runtime() {
			return;
		}
		self.emit_magnitude_functions();
		self.emit_signed_functions();
		self.emit_handle_functions();
		self.emit_int_functions();
	}

	/// Unsigned limb-array arithmetic
	fn emit_magnitude_functions(&mut self) {
		let limbs = self.limbs_ref();
		let limbs_type = self.type_manager.limbs_type;

		// big_trim(m) -> m without leading zero limbs; locals: n, trimmed
		self.runtime_function("big_trim", vec![limbs], vec![limbs], vec![ValType::I32, limbs], |_, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::ArrayLen, I::LocalSet(1)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(1), I::I32Eqz, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1), I::I32Const(1), I::I32Sub, I::ArrayGet(limbs_type), I::BrIf(1)]);
			Self::increment(f, 1, -1);
			Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(1), I::LocalGet(0), I::ArrayLen, I::I32Eq]);
			Self::emit_list(f, &[I::If(BlockType::Result(limbs)), I::LocalGet(0), I::Else]);
			Self::emit_list(f, &[I::LocalGet(1), I::ArrayNewDefault(limbs_type), I::LocalSet(2)]);
			Self::emit_list(f, &[
				I::LocalGet(2), I::I32Const(0), I::LocalGet(0), I::I32Const(0), I::LocalGet(1),
				I::ArrayCopy { array_type_index_dst: limbs_type, array_type_index_src: limbs_type },
			]);
			Self::emit_list(f, &[I::LocalGet(2), I::End]);
		});

		// mag_cmp(a, b) -> -1/0/1; locals: i, x, y
		self.runtime_function("mag_cmp", vec![limbs, limbs], vec![ValType::I32], vec![ValType::I32, ValType::I64, ValType::I64], |s, f| {
			Self::emit_list(f, &[
				I::LocalGet(0), I::ArrayLen, I::LocalGet(1), I::ArrayLen,
				I::LocalGet(0), I::ArrayLen, I::LocalGet(1), I::ArrayLen, I::I32GtU, I::Select, I::LocalSet(2),
			]);
			Self::count_down(f, 2, |f| {
				s.limb_or_zero(f, 0, I::LocalGet(2));
				f.instruction(&I::LocalSet(3));
				s.limb_or_zero(f, 1, I::LocalGet(2));
				f.instruction(&I::LocalSet(4));
				Self::emit_list(f, &[I::LocalGet(3), I::LocalGet(4), I::I64Ne, I::If(BlockType::Empty)]);
				Self::emit_list(f, &[I::I32Const(1), I::I32Const(-1), I::LocalGet(3), I::LocalGet(4), I::I64GtU, I::Select, I::Return, I::End]);
			});
			f.instruction(&I::I32Const(0));
		});

		// mag_add(a, b); locals: n, sum, i, acc
		self.runtime_function("mag_add", vec![limbs, limbs], vec![limbs], vec![ValType::I32, limbs, ValType::I32, ValType::I64], |s, f| {
			Self::emit_list(f, &[
				I::LocalGet(0), I::ArrayLen, I::LocalGet(1), I::ArrayLen,
				I::LocalGet(0), I::ArrayLen, I::LocalGet(1), I::ArrayLen, I::I32GtU, I::Select, I::LocalSet(2),
			]);
			Self::emit_list(f, &[I::LocalGet(2), I::I32Const(1), I::I32Add, I::ArrayNewDefault(limbs_type), I::LocalSet(3)]);
			Self::count_up(f, 4, 2, |f| {
				s.limb_or_zero(f, 0, I::LocalGet(4));
				s.limb_or_zero(f, 1, I::LocalGet(4));
				Self::emit_list(f, &[I::I64Add, I::LocalGet(5), I::I64Add, I::LocalSet(5)]);
				Self::emit_list(f, &[I::LocalGet(3), I::LocalGet(4), I::LocalGet(5), I::I32WrapI64]);
				s.limbs_set(f);
				Self::emit_list(f, &[I::LocalGet(5), I::I64Const(32), I::I64ShrU, I::LocalSet(5)]);
			});
			Self::emit_list(f, &[I::LocalGet(3), I::LocalGet(2), I::LocalGet(5), I::I32WrapI64]);
			s.limbs_set(f);
			f.instruction(&I::LocalGet(3));
			s.call(f, "big_trim");
		});

		// mag_sub_into(r, b): r -= b in place, requires r >= b; locals: n, i, acc, borrow
		self.runtime_function("mag_sub_into", vec![limbs, limbs], vec![], vec![ValType::I32, ValType::I32, ValType::I64, ValType::I64], |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::ArrayLen, I::LocalSet(2)]);
			Self::count_up(f, 3, 2, |f| {
				s.limb_or_zero(f, 0, I::LocalGet(3));
				s.limb_or_zero(f, 1, I::LocalGet(3));
				Self::emit_list(f, &[I::I64Sub, I::LocalGet(5), I::I64Sub, I::LocalSet(4)]);
				Self::emit_list(f, &[I::LocalGet(4), I::I64Const(0), I::I64LtS, I::I64ExtendI32U, I::LocalSet(5)]);
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(3), I::LocalGet(4), I::I32WrapI64]);
				s.limbs_set(f);
			});
		});

		// mag_sub(a, b) -> a - b, requires a >= b; locals: copy
		self.runtime_function("mag_sub", vec![limbs, limbs], vec![limbs], vec![limbs], |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::ArrayLen, I::ArrayNewDefault(limbs_type), I::LocalSet(2)]);
			Self::emit_list(f, &[
				I::LocalGet(2), I::I32Const(0), I::LocalGet(0), I::I32Const(0), I::LocalGet(0), I::ArrayLen,
				I::ArrayCopy { array_type_index_dst: limbs_type, array_type_index_src: limbs_type },
			]);
			Self::emit_list(f, &[I::LocalGet(2), I::LocalGet(1)]);
			s.call(f, "mag_sub_into");
			f.instruction(&I::LocalGet(2));
			s.call(f, "big_trim");
		});

		// mag_mul(a, b) schoolbook; locals: la, lb, product, i, j, ai, carry
		let i32s = ValType::I32;
		self.runtime_function("mag_mul", vec![limbs, limbs], vec![limbs], vec![i32s, i32s, limbs, i32s, i32s, ValType::I64, ValType::I64], |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::ArrayLen, I::LocalSet(2), I::LocalGet(1), I::ArrayLen, I::LocalSet(3)]);
			Self::emit_list(f, &[I::LocalGet(2), I::LocalGet(3), I::I32Add, I::ArrayNewDefault(limbs_type), I::LocalSet(4)]);
			Self::count_up(f, 5, 2, |f| {
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(5)]);
				s.limbs_get(f);
				Self::emit_list(f, &[I::LocalSet(7), I::I64Const(0), I::LocalSet(8), I::I32Const(0), I::LocalSet(6)]);
				Self::count_up(f, 6, 3, |f| {
					// t = ai * b[j] + product[i+j] + carry  (< 2^64)
					Self::emit_list(f, &[I::LocalGet(7), I::LocalGet(1), I::LocalGet(6)]);
					s.limbs_get(f);
					f.instruction(&I::I64Mul);
					Self::emit_list(f, &[I::LocalGet(4), I::LocalGet(5), I::LocalGet(6), I::I32Add]);
					s.limbs_get(f);
					Self::emit_list(f, &[I::I64Add, I::LocalGet(8), I::I64Add, I::LocalSet(8)]);
					Self::emit_list(f, &[I::LocalGet(4), I::LocalGet(5), I::LocalGet(6), I::I32Add, I::LocalGet(8), I::I32WrapI64]);
					s.limbs_set(f);
					Self::emit_list(f, &[I::LocalGet(8), I::I64Const(32), I::I64ShrU, I::LocalSet(8)]);
				});
				Self::emit_list(f, &[I::LocalGet(4), I::LocalGet(5), I::LocalGet(3), I::I32Add, I::LocalGet(8), I::I32WrapI64]);
				s.limbs_set(f);
			});
			f.instruction(&I::LocalGet(4));
			s.call(f, "big_trim");
		});

		// mag_divmod(a, b) -> quotient, remainder in its global; binary long division, b nonzero
		// locals: quotient, remainder, bit, n, k, carry, v
		let remainder_global = self.int_remainder_global;
		self.runtime_function("mag_divmod", vec![limbs, limbs], vec![limbs], vec![limbs, limbs, i32s, i32s, i32s, i32s, i32s], |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::ArrayLen, I::ArrayNewDefault(limbs_type), I::LocalSet(2)]);
			Self::emit_list(f, &[I::LocalGet(1), I::ArrayLen, I::I32Const(1), I::I32Add, I::LocalTee(5), I::ArrayNewDefault(limbs_type), I::LocalSet(3)]);
			Self::emit_list(f, &[I::LocalGet(0), I::ArrayLen, I::I32Const(5), I::I32Shl, I::LocalSet(4)]);
			Self::count_down(f, 4, |f| {
				// carry = bit `bit` of a
				Self::emit_list(f, &[
					I::LocalGet(0), I::LocalGet(4), I::I32Const(5), I::I32ShrU, I::ArrayGet(limbs_type),
					I::LocalGet(4), I::I32Const(31), I::I32And, I::I32ShrU, I::I32Const(1), I::I32And, I::LocalSet(7),
				]);
				// remainder = remainder << 1 | carry
				Self::emit_list(f, &[I::I32Const(0), I::LocalSet(6)]);
				Self::count_up(f, 6, 5, |f| {
					Self::emit_list(f, &[I::LocalGet(3), I::LocalGet(6), I::ArrayGet(limbs_type), I::LocalSet(8)]);
					Self::emit_list(f, &[
						I::LocalGet(3), I::LocalGet(6), I::LocalGet(8), I::I32Const(1), I::I32Shl, I::LocalGet(7), I::I32Or,
						I::ArraySet(limbs_type),
					]);
					Self::emit_list(f, &[I::LocalGet(8), I::I32Const(31), I::I32ShrU, I::LocalSet(7)]);
				});
				Self::emit_list(f, &[I::LocalGet(3), I::LocalGet(1)]);
				s.call(f, "mag_cmp");
				Self::emit_list(f, &[I::I32Const(0), I::I32GeS, I::If(BlockType::Empty), I::LocalGet(3), I::LocalGet(1)]);
				s.call(f, "mag_sub_into");
				Self::emit_list(f, &[
					I::LocalGet(2), I::LocalGet(4), I::I32Const(5), I::I32ShrU,
					I::LocalGet(2), I::LocalGet(4), I::I32Const(5), I::I32ShrU, I::ArrayGet(limbs_type),
					I::I32Const(1), I::LocalGet(4), I::I32Const(31), I::I32And, I::I32Shl, I::I32Or,
					I::ArraySet(limbs_type), I::End,
				]);
			});
			f.instruction(&I::LocalGet(3));
			s.call(f, "big_trim");
			f.instruction(&I::GlobalSet(remainder_global));
			f.instruction(&I::LocalGet(2));
			s.call(f, "big_trim");
		});
	}

	/// Sign-magnitude $BigInt arithmetic
	fn emit_signed_functions(&mut self) {
		let limbs = self.limbs_ref();
		let big = self.big_ref();
		let big_type = self.type_manager.big_int_type;
		let limbs_type = self.type_manager.limbs_type;

		// big_new(negative, limbs) normalizes: trimmed, zero never negative; locals: trimmed
		self.runtime_function("big_new", vec![ValType::I32, limbs], vec![big], vec![limbs], |s, f| {
			f.instruction(&I::LocalGet(1));
			s.call(f, "big_trim");
			Self::emit_list(f, &[I::LocalTee(2), I::ArrayLen, I::I32Const(0), I::I32Ne, I::LocalGet(0), I::I32Const(0), I::I32Ne, I::I32And]);
			Self::emit_list(f, &[I::LocalGet(2), I::StructNew(big_type)]);
		});

		// big_from_i64(x); locals: magnitude as u64
		self.runtime_function("big_from_i64", vec![ValType::I64], vec![big], vec![ValType::I64], |s, f| {
			Self::emit_list(f, &[
				I::I64Const(0), I::LocalGet(0), I::I64Sub, I::LocalGet(0), I::LocalGet(0), I::I64Const(0), I::I64LtS, I::Select, I::LocalSet(1),
			]);
			Self::emit_list(f, &[I::LocalGet(0), I::I64Const(0), I::I64LtS]);
			Self::emit_list(f, &[
				I::LocalGet(1), I::I32WrapI64, I::LocalGet(1), I::I64Const(32), I::I64ShrU, I::I32WrapI64,
				I::ArrayNewFixed { array_type_index: limbs_type, array_size: 2 },
			]);
			s.call(f, "big_new");
		});

		// big_add(a, b); locals: none
		self.runtime_function("big_add", vec![big, big], vec![big], vec![], |s, f| {
			s.negative(f, 0);
			s.negative(f, 1);
			Self::emit_list(f, &[I::I32Eq, I::If(BlockType::Result(big))]);
			s.negative(f, 0);
			s.magnitude(f, 0);
			s.magnitude(f, 1);
			s.call(f, "mag_add");
			s.call(f, "big_new");
			f.instruction(&I::Else);
			s.magnitude(f, 0);
			s.magnitude(f, 1);
			s.call(f, "mag_cmp");
			Self::emit_list(f, &[I::I32Const(0), I::I32GeS, I::If(BlockType::Result(big))]);
			for (larger, smaller) in [(0, 1), (1, 0)] {
				s.negative(f, larger);
				s.magnitude(f, larger);
				s.magnitude(f, smaller);
				s.call(f, "mag_sub");
				s.call(f, "big_new");
				if larger == 0 {
					f.instruction(&I::Else);
				}
			}
			Self::emit_list(f, &[I::End, I::End]);
		});

		self.runtime_function("big_negate", vec![big], vec![big], vec![], |s, f| {
			s.negative(f, 0);
			f.instruction(&I::I32Eqz);
			s.magnitude(f, 0);
			s.call(f, "big_new");
		});

		self.runtime_function("big_mul", vec![big, big], vec![big], vec![], |s, f| {
			s.negative(f, 0);
			s.negative(f, 1);
			f.instruction(&I::I32Xor);
			s.magnitude(f, 0);
			s.magnitude(f, 1);
			s.call(f, "mag_mul");
			s.call(f, "big_new");
		});

		// big_cmp(a, b) -> -1/0/1; locals: magnitude comparison
		self.runtime_function("big_cmp", vec![big, big], vec![ValType::I32], vec![ValType::I32], |s, f| {
			s.negative(f, 0);
			s.negative(f, 1);
			Self::emit_list(f, &[I::I32Ne, I::If(BlockType::Result(ValType::I32)), I::I32Const(-1), I::I32Const(1)]);
			s.negative(f, 0);
			Self::emit_list(f, &[I::Select, I::Else]);
			s.magnitude(f, 0);
			s.magnitude(f, 1);
			s.call(f, "mag_cmp");
			Self::emit_list(f, &[I::LocalSet(2), I::I32Const(0), I::LocalGet(2), I::I32Sub, I::LocalGet(2)]);
			s.negative(f, 0);
			Self::emit_list(f, &[I::Select, I::End]);
		});

		// big_divmod(a, b) -> quotient, or the remainder when `remainder`; truncated like i64.div_s / i64.rem_s
		let remainder_global = self.int_remainder_global;
		for (name, remainder) in [("big_quot", false), ("big_rem", true)] {
			self.runtime_function(name, vec![big, big], vec![big], vec![limbs], |s, f| {
				s.magnitude(f, 1);
				Self::emit_list(f, &[I::ArrayLen, I::I32Eqz, I::If(BlockType::Empty), I::Unreachable, I::End]);
				s.magnitude(f, 0);
				s.magnitude(f, 1);
				s.call(f, "mag_divmod");
				if remainder {
					f.instruction(&I::Drop);
					s.negative(f, 0);
					f.instruction(&I::GlobalGet(remainder_global));
				} else {
					f.instruction(&I::LocalSet(2));
					s.negative(f, 0);
					s.negative(f, 1);
					Self::emit_list(f, &[I::I32Xor, I::LocalGet(2)]);
				}
				s.call(f, "big_new");
			});
		}

		// big_to_f64(a); locals: i, acc, magnitude
		self.runtime_function("big_to_f64", vec![big], vec![ValType::F64], vec![ValType::I32, ValType::F64, limbs], |s, f| {
			s.magnitude(f, 0);
			Self::emit_list(f, &[I::LocalTee(3), I::ArrayLen, I::LocalSet(1)]);
			Self::count_down(f, 1, |f| {
				Self::emit_list(f, &[I::LocalGet(2), I::F64Const(LIMB_BASE.into()), I::F64Mul, I::LocalGet(3), I::LocalGet(1)]);
				s.limbs_get(f);
				Self::emit_list(f, &[I::F64ConvertI64U, I::F64Add, I::LocalSet(2)]);
			});
			Self::emit_list(f, &[I::LocalGet(2), I::F64Neg, I::LocalGet(2)]);
			s.negative(f, 0);
			f.instruction(&I::Select);
		});

		// big_low_i64(a): value mod 2^64 as signed i64; locals: magnitude
		self.runtime_function("big_low_i64", vec![big], vec![ValType::I64], vec![limbs], |s, f| {
			s.magnitude(f, 0);
			f.instruction(&I::LocalSet(1));
			s.limb_or_zero(f, 1, I::I32Const(0));
			s.limb_or_zero(f, 1, I::I32Const(1));
			Self::emit_list(f, &[I::I64Const(32), I::I64Shl, I::I64Or, I::I64Const(-1), I::I64Const(1)]);
			s.negative(f, 0);
			Self::emit_list(f, &[I::Select, I::I64Mul]);
		});
	}

	/// Heap of $BigInt behind Int handles
	fn emit_handle_functions(&mut self) {
		let big = self.big_ref();
		let limbs = self.limbs_ref();
		let heap_type = self.type_manager.big_heap_type;
		let heap = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(heap_type) });
		let (heap_global, count_global) = (self.int_heap_global, self.int_count_global);

		// int_store(b) -> new handle; locals: heap, grown
		self.runtime_function("int_store", vec![big], vec![ValType::I64], vec![heap, heap], |_, f| {
			Self::emit_list(f, &[I::GlobalGet(heap_global), I::LocalTee(1), I::RefIsNull]);
			Self::emit_list(f, &[
				I::If(BlockType::Result(ValType::I32)), I::I32Const(1), I::Else,
				I::GlobalGet(count_global), I::LocalGet(1), I::ArrayLen, I::I32GeU, I::End,
			]);
			f.instruction(&I::If(BlockType::Empty));
			Self::emit_list(f, &[
				I::LocalGet(1), I::RefIsNull, I::If(BlockType::Result(ValType::I32)), I::I32Const(INITIAL_HEAP_SIZE), I::Else,
				I::LocalGet(1), I::ArrayLen, I::I32Const(1), I::I32Shl, I::End,
				I::ArrayNewDefault(heap_type), I::LocalSet(2),
			]);
			Self::emit_list(f, &[
				I::LocalGet(1), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty),
				I::LocalGet(2), I::I32Const(0), I::LocalGet(1), I::I32Const(0), I::GlobalGet(count_global),
				I::ArrayCopy { array_type_index_dst: heap_type, array_type_index_src: heap_type }, I::End,
			]);
			Self::emit_list(f, &[I::LocalGet(2), I::GlobalSet(heap_global), I::LocalGet(2), I::LocalSet(1), I::End]);
			Self::emit_list(f, &[I::LocalGet(1), I::GlobalGet(count_global), I::LocalGet(0), I::ArraySet(heap_type)]);
			Self::emit_list(f, &[I::GlobalGet(count_global), I::I64ExtendI32U, I::I64Const(HANDLE_BASE), I::I64Add]);
			Self::emit_list(f, &[I::GlobalGet(count_global), I::I32Const(1), I::I32Add, I::GlobalSet(count_global)]);
		});

		// int_box(b) -> fixnum when it fits, else a handle; locals: magnitude, value
		self.runtime_function("int_box", vec![big], vec![ValType::I64], vec![limbs, ValType::I64], |s, f| {
			s.magnitude(f, 0);
			Self::emit_list(f, &[I::LocalTee(1), I::ArrayLen, I::I32Const(2), I::I32LeU, I::If(BlockType::Empty)]);
			s.limb_or_zero(f, 1, I::I32Const(0));
			s.limb_or_zero(f, 1, I::I32Const(1));
			Self::emit_list(f, &[I::I64Const(32), I::I64Shl, I::I64Or, I::LocalSet(2)]);
			s.negative(f, 0);
			Self::emit_list(f, &[
				I::If(BlockType::Empty),
				I::LocalGet(2), I::I64Const(-(FIXNUM_MIN as i64)), I::I64LeU,
				I::If(BlockType::Empty), I::I64Const(0), I::LocalGet(2), I::I64Sub, I::Return, I::End,
				I::Else,
				I::LocalGet(2), I::I64Const(FIXNUM_MAX as i64), I::I64LeU,
				I::If(BlockType::Empty), I::LocalGet(2), I::Return, I::End,
				I::End, I::End,
			]);
			f.instruction(&I::LocalGet(0));
			s.call(f, "int_store");
		});

		// int_unbox(x) -> $BigInt for any Int
		self.runtime_function("int_unbox", vec![ValType::I64], vec![big], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0]);
			Self::emit_list(f, &[I::If(BlockType::Result(big)), I::LocalGet(0)]);
			s.call(f, "big_from_i64");
			f.instruction(&I::Else);
			s.emit_heap_get(f, 0);
			f.instruction(&I::End);
		});
	}

	/// Slow paths behind the inline checks, all (Int...) -> Int
	fn emit_int_functions(&mut self) {
		let (i64t, i32t, f64t) = (ValType::I64, ValType::I32, ValType::F64);
		let big = self.big_ref();
		let any = Ref(RefType { nullable: true, heap_type: crate::type_kinds::any_heap_type() });
		let i64_box = self.type_manager.i64_box_type;
		let big_type = self.type_manager.big_int_type;

		let unbox_both = |s: &Self, f: &mut Function| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "int_unbox");
			f.instruction(&I::LocalGet(1));
			s.call(f, "int_unbox");
		};

		self.runtime_function("int_add_slow", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			unbox_both(s, f);
			s.call(f, "big_add");
			s.call(f, "int_box");
		});

		self.runtime_function("int_sub_slow", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			unbox_both(s, f);
			s.call(f, "big_negate");
			s.call(f, "big_add");
			s.call(f, "int_box");
		});

		// int_mul_slow: fixnums that overflow only the fast test stay unallocated; locals: product
		self.runtime_function("int_mul_slow", vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			s.emit_fixnum_test(f, &[0, 1]);
			f.instruction(&I::If(BlockType::Empty));
			Self::emit_list(f, &[I::LocalGet(0), I::I64Eqz, I::If(BlockType::Empty), I::I64Const(0), I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1), I::I64Mul, I::LocalTee(2), I::LocalGet(0), I::I64DivS, I::LocalGet(1), I::I64Eq]);
			s.emit_fixnum_test(f, &[2]);
			Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty), I::LocalGet(2), I::Return, I::End, I::End]);
			unbox_both(s, f);
			s.call(f, "big_mul");
			s.call(f, "int_box");
		});

		// int_mul(a, b): checked multiply with the inline fast test, used by int_pow
		self.runtime_function("int_mul", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			Self::emit_mul_fast_test(f, &[0, 1]);
			Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(0), I::LocalGet(1), I::I64Mul, I::Else, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "int_mul_slow");
			f.instruction(&I::End);
		});

		self.runtime_function("int_neg_slow", vec![i64t], vec![i64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "int_unbox");
			s.call(f, "big_negate");
			s.call(f, "int_box");
		});

		// int_abs_slow(x): same magnitude, never negative; locals: unboxed
		self.runtime_function("int_abs_slow", vec![i64t], vec![i64t], vec![big], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "int_unbox");
			f.instruction(&I::LocalSet(1));
			f.instruction(&I::I32Const(0));
			s.magnitude(f, 1);
			f.instruction(&I::StructNew(big_type));
			s.call(f, "int_box");
		});

		// int_cmp(a, b) -> -1/0/1
		self.runtime_function("int_cmp", vec![i64t, i64t], vec![i32t], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0, 1]);
			Self::emit_list(f, &[
				I::If(BlockType::Result(i32t)),
				I::LocalGet(0), I::LocalGet(1), I::I64GtS, I::LocalGet(0), I::LocalGet(1), I::I64LtS, I::I32Sub,
				I::Else,
			]);
			unbox_both(s, f);
			s.call(f, "big_cmp");
			f.instruction(&I::End);
		});

		// int_quot_slow / int_rem_slow: truncated division
		for (name, big_op) in [("int_quot_slow", "big_quot"), ("int_rem_slow", "big_rem")] {
			self.runtime_function(name, vec![i64t, i64t], vec![i64t], vec![], |s, f| {
				unbox_both(s, f);
				s.call(f, big_op);
				s.call(f, "int_box");
			});
		}

		self.runtime_function("int_to_f64", vec![i64t], vec![f64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "int_unbox");
			s.call(f, "big_to_f64");
		});

		// int_from_i64(x): any machine i64 as Int
		self.runtime_function("int_from_i64", vec![i64t], vec![i64t], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0]);
			Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(0), I::Else, I::LocalGet(0)]);
			s.call(f, "big_from_i64");
			s.call(f, "int_store");
			f.instruction(&I::End);
		});

		// int_to_i64_wrap(x): Int mod 2^64 as a machine i64
		self.runtime_function("int_to_i64_wrap", vec![i64t], vec![i64t], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0]);
			Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(0), I::Else, I::LocalGet(0)]);
			s.call(f, "int_unbox");
			s.call(f, "big_low_i64");
			f.instruction(&I::End);
		});

		// int_xor(a, b): bitwise on the two's complement i64 values
		self.runtime_function("int_xor", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "int_to_i64_wrap");
			f.instruction(&I::LocalGet(1));
			s.call(f, "int_to_i64_wrap");
			f.instruction(&I::I64Xor);
			s.call(f, "int_from_i64");
		});

		// int_pow(base, exponent) by squaring; negative or huge exponents trap; locals: result
		self.runtime_function("int_pow", vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			Self::emit_list(f, &[I::LocalGet(1), I::I64Const(0), I::I64LtS, I::If(BlockType::Empty), I::Unreachable, I::End]);
			s.emit_fixnum_test(f, &[1]);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::Unreachable, I::End]);
			Self::emit_list(f, &[I::I64Const(1), I::LocalSet(2), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(1), I::I64Eqz, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(1), I::I64Const(1), I::I64And, I::I32WrapI64, I::If(BlockType::Empty), I::LocalGet(2), I::LocalGet(0)]);
			s.call(f, "int_mul");
			Self::emit_list(f, &[I::LocalSet(2), I::End]);
			Self::emit_list(f, &[I::LocalGet(1), I::I64Const(1), I::I64ShrS, I::LocalTee(1), I::I64Eqz, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(0)]);
			s.call(f, "int_mul");
			Self::emit_list(f, &[I::LocalSet(0), I::Br(0), I::End, I::End, I::LocalGet(2)]);
		});

		// int_from_payload(data) -> Int for the data field of an Int node
		self.runtime_function("int_from_payload", vec![any], vec![i64t], vec![], |s, f| {
			Self::emit_list(f, &[
				I::LocalGet(0), I::RefTestNonNull(HeapType::Concrete(i64_box)), I::If(BlockType::Result(i64t)),
				I::LocalGet(0), I::RefCastNonNull(HeapType::Concrete(i64_box)),
				I::StructGet { struct_type_index: i64_box, field_index: 0 },
				I::Else,
				I::LocalGet(0), I::RefCastNonNull(HeapType::Concrete(big_type)),
			]);
			s.call(f, "int_box");
			f.instruction(&I::End);
		});
	}
}
