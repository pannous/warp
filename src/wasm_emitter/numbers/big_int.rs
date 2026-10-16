//! Unbounded Int: an i64 fast path that promotes to BigInt on overflow.
//!
//! An Int is one i64 everywhere (locals, params, returns, globals). Values in the fixnum
//! range `[FIXNUM_MIN, FIXNUM_MAX] = [-(2^62-1), 2^62]` stand for themselves. Every other
//! i64 is a handle `i64::MIN + index` into the `$Numbers` heap of normalized `$BigInt`s
//! and exact `$Ratio`s (see exact.rs): an Int value is any exact number.
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

use super::super::WasmGcEmitter;
use crate::extensions::numbers::Number;
use crate::node::Node;
use crate::numbers::fixed_width::FixedWidth;
use crate::operators::Op;
use num_bigint::{BigInt, Sign};
use std::collections::HashSet;
use wasm_encoder::*;
use Instruction as I;
use ValType::Ref;

mod magnitudes;
mod slow_paths;

pub const INT_RUNTIME: &str = "int_runtime";
/// Marks a program that calculates: exact_cmp then carries its small shortcut
pub const EXACT_ARITHMETIC: &str = "exact_arithmetic";
/// The exported Int operations a host builds an exact number beyond the fixnums with (tasks.rs Builders, host.js
/// exactHandle): shift, add, subtract, divide
pub const EXACT_BUILDERS: [&str; 4] = ["int_shift_left", "exact_add", "exact_sub", "exact_div"];
pub const FIXNUM_OFFSET: i64 = (1 << 62) - 1;
pub const FIXNUM_MIN: i128 = -((1 << 62) - 1);
pub const FIXNUM_MAX: i128 = 1 << 62;
const HANDLE_BASE: i64 = i64::MIN;
/// Every integer within ±2^53 is an f64 exactly
const F64_EXACT_LIMIT: i64 = 1 << 53;
/// A small ratio lives in its handle and never on the heap, which int_store never frees: handles from INLINE_RATIO_BASE
/// on hold `((numerator + INLINE_NUMERATOR_BIAS) << INLINE_DENOMINATOR_BITS) | denominator`, a numerator of 31 bits and
/// a denominator of 30; the heap's handles stay below (2^61 of them)
const INLINE_RATIO_SPAN: i64 = 1 << 61;
const INLINE_RATIO_BASE: i64 = HANDLE_BASE + INLINE_RATIO_SPAN;
const INLINE_DENOMINATOR_BITS: i64 = 30;
const INLINE_NUMERATOR_BIAS: i64 = 1 << INLINE_DENOMINATOR_BITS;
const INLINE_NUMERATOR_LIMIT: i64 = 1 << (INLINE_DENOMINATOR_BITS + 1);
/// Operands in [-2^31, 2^31) multiply without leaving the fixnum range
const MUL_FAST_BIAS: i64 = 1 << 31;
const MUL_FAST_BITS: u32 = 32;
/// Integers in [-2^53, 2^53) are exact f64s
const FLOAT_EXACT_BITS: u32 = 54;
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

/// The f64 that is exactly this number: an integer within ±2^53, a ratio over a power of two
fn exact_f64(number: &Number) -> Option<f64> {
	let within = |n: i64| n.unsigned_abs() <= F64_EXACT_LIMIT as u64;
	match *number {
		Number::Int(n) if within(n) => Some(n as f64),
		// 6/4: the odd part of the denominator divides the numerator, what is left is over a power of two
		Number::Quotient(n, d) if d > 0 => {
			let odd = d >> d.trailing_zeros();
			(n % odd == 0 && within(n / odd)).then(|| (n / odd) as f64 / (d / odd) as f64)
		}
		// a decimal literal means its digits (0.5 is 5/10): exact as an f64 when the f64 is that decimal (0.1 is not)
		Number::Float(value) => {
			let (numerator, denominator) = super::super::numbers::exact::decimal_fraction(value);
			let (numerator, denominator) = (i64::try_from(&numerator).ok()?, i64::try_from(&denominator).ok()?);
			exact_f64(&Number::Quotient(numerator, denominator)).filter(|exact| *exact == value)
		}
		_ => None,
	}
}

pub fn is_fixnum(n: i64) -> bool {
	is_fixnum_range(Some((n as i128, n as i128)))
}

fn combine(left: IntRange, right: IntRange, op: impl Fn(i128, i128) -> Option<i128>) -> IntRange {
	let ((a, b), (c, d)) = (left?, right?);
	let corners = [op(a, c)?, op(a, d)?, op(b, c)?, op(b, d)?];
	Some((*corners.iter().min()?, *corners.iter().max()?))
}

/// Euclidean `a % b` on machine i64s in locals a, b (r is scratch): `r = a rem_s b; r + ((r >> 63) & |b|)`,
/// so 0 ≤ r < |b| as in mathematics (-7 % 3 → 2, 7 % -3 → 1); `rem` keeps the truncated i64.rem_s
pub(in crate::wasm_emitter) fn euclidean_remainder(a: u32, b: u32, r: u32) -> Vec<Instruction<'static>> {
	let sign_mask = |local| [I::LocalGet(local), I::I64Const(63), I::I64ShrS];
	let mut code = vec![I::LocalGet(a), I::LocalGet(b), I::I64RemS, I::LocalTee(r)];
	code.extend(sign_mask(r));
	code.push(I::LocalGet(b));
	code.extend(sign_mask(b));
	code.push(I::I64Xor);
	code.extend(sign_mask(b));
	code.extend([I::I64Sub, I::I64And, I::I64Add]);
	code
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
			Node::Symbol(name) if self.bounded_counters.contains(name) => Some((0, i32::MAX as i128)),
			Node::Key(left, op, right) if left.is_empty_node() && (op.is_prefix() || *op == Op::Hash) => match op {
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
					// the dividend must be a proven integer: a ratio % n is a ratio
					Op::Mod => l.and(r).map(|(low, high)| (0, (low.abs().max(high.abs()) - 1).max(0))),
					Op::Rem => l.and(r).map(|(low, high)| {
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

	pub(in crate::wasm_emitter) fn scratch(&self, index: u32) -> u32 {
		self.int_scratch + index
	}

	/// Push i32 1 when every listed local holds a fixnum: `((x + OFFSET) | ...) >= 0`
	pub(in crate::wasm_emitter) fn emit_fixnum_test(&self, func: &mut Function, locals: &[u32]) {
		Self::emit_offset_locals_test(func, locals, FIXNUM_OFFSET, &[I::I64Const(0), I::I64GeS]);
	}

	fn unproven(&self, candidates: &[(u32, IntRange)]) -> Vec<u32> {
		candidates.iter().filter(|(_, range)| !is_fixnum_range(*range)).map(|(local, _)| *local).collect()
	}

	/// `a << n` and `a >> n` on exact Ints; a float operand is refused by the operand emitters
	pub(crate) fn emit_shift(&mut self, func: &mut Function, left: &Node, op: &Op, right: &Node) {
		self.emit_numeric_value(func, left);
		self.emit_numeric_value(func, right);
		self.emit_call(func, if *op == Op::Shl { "int_shift_left" } else { "int_shift_right" });
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
		match op {
			Op::Div => return self.emit_call(func, "exact_div"),
			Op::Pow => return self.emit_call(func, "exact_pow"),
			Op::Xor => return self.emit_call(func, "int_xor"),
			Op::BitAnd => return self.emit_call(func, "int_and"),
			Op::BitOr => return self.emit_call(func, "int_or"),
			_ => {}
		}
		let (a, b, r) = (self.scratch(0), self.scratch(1), self.scratch(2));
		Self::emit_list(func, &[I::LocalSet(b), I::LocalSet(a)]);
		match op {
			Op::Add | Op::Sub => {
				Self::emit_list(
					func,
					&[
						I::LocalGet(a),
						I::LocalGet(b),
						if *op == Op::Add { I::I64Add } else { I::I64Sub },
						I::LocalSet(r),
					],
				);
				let unproven = self.unproven(&[(r, result), (a, left), (b, right)]);
				self.emit_fixnum_test(func, &unproven);
				self.emit_fast_or_slow(func, &[I::LocalGet(r)], if *op == Op::Add { "exact_add" } else { "exact_sub" });
			}
			Op::Mul => {
				let small = |range| fits(range, -MUL_FAST_BIAS as i128, MUL_FAST_BIAS as i128 - 1);
				let unproven: Vec<u32> =
					[(a, left), (b, right)].iter().filter(|(_, range)| !small(*range)).map(|(l, _)| *l).collect();
				Self::emit_mul_fast_test(func, &unproven);
				self.emit_fast_or_slow(func, &[I::LocalGet(a), I::LocalGet(b), I::I64Mul], "exact_mul");
			}
			Op::Mod => {
				self.emit_nonzero_divisor(func, b, right);
				let unproven = self.unproven(&[(a, left), (b, right)]);
				self.emit_fixnum_test(func, &unproven);
				let slow = self.division_runtime(super::exact::INT_MOD, "exact_mod");
				self.emit_fast_or_slow(func, &euclidean_remainder(a, b, r), slow);
			}
			Op::Rem => {
				self.emit_nonzero_divisor(func, b, right);
				let unproven = self.unproven(&[(a, left), (b, right)]);
				self.emit_fixnum_test(func, &unproven);
				let slow = self.division_runtime("int_rem", "exact_rem");
				self.emit_fast_or_slow(func, &[I::LocalGet(a), I::LocalGet(b), I::I64RemS], slow);
			}
			_ => unreachable!("not an Int operator: {:?}", op),
		}
	}

	/// Stack [a, b] (Ints) → [a // b], the Euclidean quotient: (a - a % b) / b is exact on fixnums, else int_euclid_div or exact_euclid_div
	pub(in crate::wasm_emitter) fn emit_euclidean_quotient(
		&mut self,
		func: &mut Function,
		left: IntRange,
		right: IntRange,
	) {
		if !self.int_runtime() {
			return self.emit_call(func, "exact_euclid_div");
		}
		let (a, b, r) = (self.scratch(0), self.scratch(1), self.scratch(2));
		Self::emit_list(func, &[I::LocalSet(b), I::LocalSet(a)]);
		self.emit_nonzero_divisor(func, b, right);
		let unproven = self.unproven(&[(a, left), (b, right)]);
		self.emit_fixnum_test(func, &unproven);
		let mut quotient = vec![I::LocalGet(a)];
		quotient.extend(euclidean_remainder(a, b, r));
		quotient.extend([I::I64Sub, I::LocalGet(b), I::I64DivS]);
		let slow = self.division_runtime(super::exact::INT_EUCLID_DIV, "exact_euclid_div");
		self.emit_fast_or_slow(func, &quotient, slow);
	}

	/// `a % 0` fails with the named error divide_by_zero (try catches it), never the engine's divide trap; skipped when the
	/// divisor's range excludes 0
	fn emit_nonzero_divisor(&mut self, func: &mut Function, divisor: u32, range: IntRange) {
		if range.is_some_and(|(low, high)| low > 0 || high < 0) {
			return;
		}
		Self::emit_list(func, &[I::LocalGet(divisor), I::I64Eqz]);
		self.emit_fail_if(func, super::super::lists::list_ops::DIVIDE_BY_ZERO);
	}

	fn emit_scratch_call(&mut self, func: &mut Function, name: &'static str) {
		Self::emit_list(func, &[I::LocalGet(self.scratch(0)), I::LocalGet(self.scratch(1))]);
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

	/// The slow path of `%`, `rem` or `//`: `exact` when ratios may reach it, else `int` (emit_module checks that)
	fn division_runtime(&mut self, int: &'static str, exact: &'static str) -> &'static str {
		if self.ratio_division {
			return exact;
		}
		self.int_division = true;
		int
	}

	/// Push i32 1 when all locals are in [-2^31, 2^31)
	fn emit_mul_fast_test(func: &mut Function, locals: &[u32]) {
		Self::emit_signed_bits_test(func, locals, MUL_FAST_BITS);
	}

	/// Push i32 1 when all locals fit in `bits` signed bits: `((x + 2^(bits-1)) | ...) >>u bits == 0`
	fn emit_signed_bits_test(func: &mut Function, locals: &[u32], bits: u32) {
		Self::emit_offset_locals_test(
			func,
			locals,
			1 << (bits - 1),
			&[I::I64Const(bits as i64), I::I64ShrU, I::I64Eqz],
		);
	}

	/// Push i32 1 for no locals, else `(x + offset) | (y + offset) | ...` followed by `test`
	fn emit_offset_locals_test(func: &mut Function, locals: &[u32], offset: i64, test: &[Instruction]) {
		if locals.is_empty() {
			func.instruction(&I::I32Const(1));
			return;
		}
		for (i, local) in locals.iter().enumerate() {
			Self::emit_list(func, &[I::LocalGet(*local), I::I64Const(offset), I::I64Add]);
			if i > 0 {
				func.instruction(&I::I64Or);
			}
		}
		Self::emit_list(func, test);
	}

	/// Stack [a, b] (Ints) → [f64]: a/b rounded once. Where both are exact f64s and b is not 0, IEEE division rounds
	/// their quotient correctly, without allocating the exact one (card exact-div); otherwise exact_div, which fails
	/// for b = 0 as before, then exact_to_f64
	pub(crate) fn emit_float_quotient(&mut self, func: &mut Function) {
		let (a, b) = (self.scratch(0), self.scratch(1));
		Self::emit_list(func, &[I::LocalSet(b), I::LocalSet(a)]);
		Self::emit_signed_bits_test(func, &[a, b], FLOAT_EXACT_BITS);
		for instruction in [
			I::LocalGet(b),
			I::I64Const(0),
			I::I64Ne,
			I::I32And,
			I::If(BlockType::Result(ValType::F64)),
			I::LocalGet(a),
			I::F64ConvertI64S,
			I::LocalGet(b),
			I::F64ConvertI64S,
			I::F64Div,
			I::Else,
			I::LocalGet(a),
			I::LocalGet(b),
		] {
			func.instruction(&instruction);
		}
		self.emit_call(func, "exact_div");
		self.emit_int_to_f64(func, None);
		func.instruction(&I::End);
	}

	fn emit_machine_int_op(&mut self, func: &mut Function, op: &Op) {
		match op {
			Op::Add => func.instruction(&I::I64Add),
			Op::Sub => func.instruction(&I::I64Sub),
			Op::Mul => func.instruction(&I::I64Mul),
			Op::Div => func.instruction(&I::I64DivS),
			Op::Mod => {
				let (a, b, r) = (self.scratch(0), self.scratch(1), self.scratch(2));
				Self::emit_list(func, &[I::LocalSet(b), I::LocalSet(a)]);
				Self::emit_list(func, &euclidean_remainder(a, b, r));
				func
			}
			Op::Rem => func.instruction(&I::I64RemS),
			Op::Xor => func.instruction(&I::I64Xor),
			Op::BitAnd => func.instruction(&I::I64And),
			Op::BitOr => func.instruction(&I::I64Or),
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
		self.emit_fast_or_unary_slow(func, a, &[I::LocalGet(a)], "exact_to_i64_wrap", ValType::I64);
	}

	/// Raw machine i64 → Int (promotes values outside the fixnum range)
	pub(crate) fn emit_int_from_machine(&mut self, func: &mut Function) {
		if !self.int_runtime() {
			return;
		}
		let r = self.scratch(2);
		self.emit_int_from_machine_in(func, r);
	}

	/// The same in a runtime function, with its own i64 local
	pub(crate) fn emit_int_from_machine_in(&mut self, func: &mut Function, local: u32) {
		if !self.int_runtime() {
			return;
		}
		func.instruction(&I::LocalSet(local));
		self.emit_fixnum_test(func, &[local]);
		self.emit_fast_or_unary_slow(func, local, &[I::LocalGet(local)], "int_from_i64", ValType::I64);
	}

	fn emit_fast_or_unary_slow(
		&mut self,
		func: &mut Function,
		local: u32,
		fast: &[Instruction],
		slow: &'static str,
		result: ValType,
	) {
		func.instruction(&I::If(BlockType::Result(result)));
		for instruction in fast {
			func.instruction(instruction);
		}
		Self::emit_list(func, &[I::Else, I::LocalGet(local)]);
		self.emit_call(func, slow);
		func.instruction(&I::End);
	}

	/// Stack [x] → [-x]
	pub(crate) fn emit_int_neg(&mut self, func: &mut Function, range: IntRange) {
		let negated = range.map(|(low, high)| (-high, -low));
		if !self.int_runtime() || (is_fixnum_range(range) && is_fixnum_range(negated)) {
			Self::emit_list(func, &[I::I64Const(-1), I::I64Mul]);
			return;
		}
		if self.wrapping_ints {
			self.emit_int_to_machine(func);
			Self::emit_list(func, &[I::I64Const(-1), I::I64Mul]);
			return self.emit_int_from_machine(func);
		}
		let (a, r) = (self.scratch(0), self.scratch(2));
		Self::emit_list(func, &[I::LocalTee(a), I::I64Const(-1), I::I64Mul, I::LocalSet(r)]);
		let unproven = self.unproven(&[(r, None), (a, range)]);
		self.emit_fixnum_test(func, &unproven);
		self.emit_fast_or_unary_slow(func, a, &[I::LocalGet(r)], "exact_neg", ValType::I64);
	}

	/// Stack [x] → [|x|]; |fixnum| is always a fixnum
	pub(crate) fn emit_int_abs(&mut self, func: &mut Function, range: IntRange) {
		let a = self.scratch(0);
		func.instruction(&I::LocalSet(a));
		#[rustfmt::skip]
		let fast = [
			I::I64Const(0), I::LocalGet(a), I::I64Sub, I::LocalGet(a), I::LocalGet(a), I::I64Const(0), I::I64LtS,
			I::Select,
		];
		if !self.int_runtime() || is_fixnum_range(range) {
			for instruction in &fast {
				func.instruction(instruction);
			}
			return;
		}
		self.emit_fixnum_test(func, &[a]);
		self.emit_fast_or_unary_slow(func, a, &fast, "exact_abs", ValType::I64);
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
		Self::emit_list(func, &[I::LocalSet(b), I::LocalSet(a)]);
		let unproven = self.unproven(&[(a, left), (b, right)]);
		self.emit_fixnum_test(func, &unproven);
		Self::emit_list(
			func,
			&[I::If(BlockType::Result(ValType::I32)), I::LocalGet(a), I::LocalGet(b), machine, I::Else],
		);
		self.emit_scratch_call(func, "exact_cmp");
		Self::emit_list(func, &[I::I32Const(0), Self::i32_compare(op), I::End]);
	}

	/// The fixed width (`byte`, `int8` …) the variable `node` was declared with
	pub(in crate::wasm_emitter) fn declared_fixed_width(&self, node: &Node) -> Option<&'static FixedWidth> {
		let Node::Symbol(name) = node.drop_meta() else {
			return None;
		};
		let type_node = self.scope.lookup(name)?.type_node.as_ref()?;
		crate::numbers::fixed_width::fixed_width(&type_node.name())
	}

	/// Is the variable `node` declared a whole-number type (`int`, `long`, `byte` …, not `exact`)
	pub(in crate::wasm_emitter) fn declared_whole(&self, node: &Node) -> bool {
		let Node::Symbol(name) = node.drop_meta() else {
			return false;
		};
		let type_node = self.scope.lookup(name).and_then(|local| local.type_node.as_ref());
		type_node.is_some_and(|type_node| crate::analyzer::is_whole_type(&type_node.name()))
	}

	/// Stack [Int] → [Int]: traps in INT_NOT_WHOLE when the Int is a ratio (`x:int = 3; x += 0.5`)
	pub(in crate::wasm_emitter) fn emit_whole_check(&mut self, func: &mut Function) {
		let value = self.scratch(2);
		func.instruction(&I::LocalSet(value));
		self.emit_require_whole(func, value, super::super::lists::list_ops::INT_NOT_WHOLE);
		func.instruction(&I::LocalGet(value));
	}

	/// Stack [Int] → [Int] before storing to `target`: traps when the Int is no whole number or does not fit its declared
	/// fixed width
	pub(in crate::wasm_emitter) fn emit_fits_declared(&mut self, func: &mut Function, target: &Node) {
		if self.declared_whole(target) {
			self.emit_whole_check(func);
		}
		let width = self.declared_fixed_width(target);
		self.emit_fixed_width_check(func, width);
	}

	/// Stack [Int] → [Int], trapping in `overflow_of_<type>` when the Int leaves the range of `width`
	pub(in crate::wasm_emitter) fn emit_fixed_width_check(
		&mut self,
		func: &mut Function,
		width: Option<&'static FixedWidth>,
	) {
		let Some(width) = width else { return };
		let value = self.scratch(2);
		func.instruction(&I::LocalSet(value));
		let below = self.emit_bound_violation(func, value, Op::Lt, width.low);
		let above = self.emit_bound_violation(func, value, Op::Gt, width.high);
		if below && above {
			func.instruction(&I::I32Or);
		}
		if below || above {
			self.emit_fail_if(func, width.trap);
		}
		func.instruction(&I::LocalGet(value));
	}

	/// Push i32 `value op bound`; without the Int runtime every Int is a machine i64, so a bound beyond i64 holds trivially
	fn emit_bound_violation(&mut self, func: &mut Function, value: u32, op: Op, bound: i128) -> bool {
		if !self.int_runtime() && i64::try_from(bound).is_err() {
			return false;
		}
		func.instruction(&I::LocalGet(value));
		self.emit_int_literal(func, &BigInt::from(bound));
		self.emit_int_compare(func, &op, None, Some((bound, bound)));
		true
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
		self.emit_fast_or_unary_slow(func, a, &[I::LocalGet(a), I::F64ConvertI64S], "exact_to_f64", ValType::F64);
	}

	/// `col + 0.5` where a float uses it (`col + 0.5 - x` of a float x): IEEE rounds an operation on exact f64 operands
	/// once, as converting the exact result does, so a variable holding an integer within ±2^53 and a constant an f64
	/// holds exactly take the f64 operation; any other value the exact one (decision exact-decimals: the value stays
	/// exact up to the float, a ratio per pixel made circle five times slower than with floats)
	pub(crate) fn emit_exact_operation_as_f64(
		&mut self,
		func: &mut Function,
		node: &Node,
		left: &Node,
		op: &Op,
		right: &Node,
	) -> bool {
		if !matches!(op, Op::Add | Op::Sub | Op::Mul) || !self.int_runtime() {
			return false;
		}
		let (variable, constant) = match (left.drop_meta(), right.drop_meta()) {
			(Node::Symbol(_), Node::Number(number)) => (left, number),
			(Node::Number(number), Node::Symbol(_)) => (right, number),
			_ => return false,
		};
		let Some(constant) = exact_f64(constant) else {
			return false;
		};
		let held = self.scratch(0);
		self.emit_numeric_value(func, variable);
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalSet(held),
			I::LocalGet(held), I::I64Const(F64_EXACT_LIMIT), I::I64Add, I::I64Const(2 * F64_EXACT_LIMIT), I::I64LeU,
			I::If(BlockType::Result(ValType::F64)),
		]);
		for side in [left, right] {
			if std::ptr::eq(side, variable) {
				Self::emit_list(func, &[I::LocalGet(held), I::F64ConvertI64S]);
			} else {
				func.instruction(&I::F64Const(Ieee64::new(constant.to_bits())));
			}
		}
		self.emit_float_arithmetic(func, op);
		func.instruction(&I::Else);
		self.emit_numeric_value(func, node);
		let range = self.int_range(node);
		self.emit_int_to_f64(func, range);
		func.instruction(&I::End);
		true
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
		self.emit_number_constant(func, number, &BigInt::from(1));
	}

	/// An Int literal built anew: a fixnum, else a $BigInt boxed into a new handle
	pub(in crate::wasm_emitter) fn emit_built_int_literal(&mut self, func: &mut Function, number: &BigInt) {
		if let Some(n) = i64::try_from(number).ok().filter(|n| is_fixnum(*n)) {
			func.instruction(&I::I64Const(n));
			return;
		}
		let (sign, limbs) = number.to_u32_digits();
		func.instruction(&I::I32Const((sign == Sign::Minus) as i32));
		for limb in &limbs {
			func.instruction(&I::I32Const(*limb as i32));
		}
		Self::emit_list(
			func,
			&[
				I::ArrayNewFixed { array_type_index: self.type_manager.limbs_type, array_size: limbs.len() as u32 },
				I::StructNew(self.type_manager.big_int_type),
			],
		);
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
		self.int_heap_global =
			self.declare_global_of(Ref(heap), ConstExpr::ref_null(HeapType::Concrete(self.type_manager.big_heap_type)));
		self.int_count_global = self.declare_global_of(ValType::I32, ConstExpr::i32_const(0));
	}

	/// Push the anyref payload for the Int in `local`
	pub(crate) fn emit_int_payload(&self, func: &mut Function, local: u32) {
		if !self.int_runtime() {
			Self::emit_list(func, &[I::LocalGet(local), I::StructNew(self.type_manager.i64_box_type)]);
			return;
		}
		let any = BlockType::Result(Ref(RefType { nullable: true, heap_type: crate::type_kinds::any_heap_type() }));
		let i64_box = self.type_manager.i64_box_type;
		self.emit_fixnum_test(func, &[local]);
		Self::emit_list(func, &[I::If(any), I::LocalGet(local), I::StructNew(i64_box), I::Else]);
		self.emit_inline_ratio_test(func, local);
		func.instruction(&I::If(any));
		self.emit_inline_numerator(func, local);
		func.instruction(&I::StructNew(i64_box));
		self.emit_inline_denominator(func, local);
		Self::emit_list(func, &[I::StructNew(i64_box), I::StructNew(self.type_manager.ratio_type), I::Else]);
		self.emit_heap_get(func, local);
		Self::emit_list(func, &[I::End, I::End]);
	}

	/// Push i32 1 when the Int in `local` is a ratio inside its handle (INLINE_RATIO_BASE)
	pub(in crate::wasm_emitter) fn emit_inline_ratio_test(&self, func: &mut Function, local: u32) {
		Self::emit_list(
			func,
			&[I::LocalGet(local), I::I64Const(INLINE_RATIO_BASE), I::I64Sub, I::I64Const(INLINE_RATIO_SPAN), I::I64LtU],
		);
	}

	pub(in crate::wasm_emitter) fn emit_inline_numerator(&self, func: &mut Function, local: u32) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(local), I::I64Const(INLINE_RATIO_BASE), I::I64Sub, I::I64Const(INLINE_DENOMINATOR_BITS),
			I::I64ShrU, I::I64Const(INLINE_NUMERATOR_BIAS), I::I64Sub,
		]);
	}

	pub(in crate::wasm_emitter) fn emit_inline_denominator(&self, func: &mut Function, local: u32) {
		Self::emit_list(func, &[I::LocalGet(local), I::I64Const(INLINE_NUMERATOR_BIAS - 1), I::I64And]);
	}

	/// Push i32 1 when numerator and denominator (Int locals of a normalized ratio) fit inside a handle
	pub(in crate::wasm_emitter) fn emit_inline_ratio_fits(
		&self,
		func: &mut Function,
		numerator: u32,
		denominator: u32,
	) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(numerator), I::I64Const(INLINE_NUMERATOR_BIAS), I::I64Add, I::I64Const(INLINE_NUMERATOR_LIMIT),
			I::I64LtU, I::LocalGet(denominator), I::I64Const(INLINE_NUMERATOR_BIAS), I::I64LtU, I::I32And,
		]);
	}

	/// Push the handle of the ratio numerator/denominator that emit_inline_ratio_fits accepted
	pub(in crate::wasm_emitter) fn emit_inline_ratio(&self, func: &mut Function, numerator: u32, denominator: u32) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(numerator), I::I64Const(INLINE_NUMERATOR_BIAS), I::I64Add, I::I64Const(INLINE_DENOMINATOR_BITS),
			I::I64Shl, I::LocalGet(denominator), I::I64Or, I::I64Const(INLINE_RATIO_BASE), I::I64Add,
		]);
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
			Self::emit_list(f, &[I::RefNull(HeapType::Concrete(node_type)), I::StructNew(node_type)]);
		});
		self.export_runtime_function("new_int");
	}

	pub(in crate::wasm_emitter) fn emit_heap_get(&self, func: &mut Function, handle_local: u32) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::GlobalGet(self.int_heap_global), I::LocalGet(handle_local), I::I64Const(HANDLE_BASE), I::I64Sub,
			I::I32WrapI64, I::ArrayGet(self.type_manager.big_heap_type),
		]);
	}

	fn emit_heap_get_big(&self, func: &mut Function, handle_local: u32) {
		self.emit_heap_get(func, handle_local);
		func.instruction(&I::RefCastNullable(HeapType::Concrete(self.type_manager.big_int_type)));
	}

	/// Stack [anyref payload of an Int node] → [Int]
	pub(crate) fn emit_int_from_payload(&mut self, func: &mut Function) {
		if self.int_runtime() {
			self.emit_call(func, "int_from_payload");
			return;
		}
		Self::emit_list(
			func,
			&[
				I::RefCastNonNull(HeapType::Concrete(self.type_manager.i64_box_type)),
				I::StructGet { struct_type_index: self.type_manager.i64_box_type, field_index: 0 },
			],
		);
	}

	// ═══════════════════════════════════════════════════════════════════════
	// Runtime functions (emitted once, only when the program needs them)
	// ═══════════════════════════════════════════════════════════════════════

	pub(in crate::wasm_emitter) fn call(&self, func: &mut Function, name: &str) {
		func.instruction(&I::Call(self.func_index(name)));
	}

	fn limbs_get(&self, func: &mut Function) {
		Self::emit_list(func, &[I::ArrayGet(self.type_manager.limbs_type), I::I64ExtendI32U]);
	}

	fn limbs_set(&self, func: &mut Function) {
		func.instruction(&I::ArraySet(self.type_manager.limbs_type));
	}

	/// Push limb `index_local` of `limbs_local` as u64, 0 past the end
	fn limb_or_zero(&self, func: &mut Function, limbs_local: u32, index: Instruction) {
		Self::emit_list(
			func,
			&[
				index.clone(),
				I::LocalGet(limbs_local),
				I::ArrayLen,
				I::I32LtU,
				I::If(BlockType::Result(ValType::I64)),
				I::LocalGet(limbs_local),
				index,
			],
		);
		self.limbs_get(func);
		Self::emit_list(func, &[I::Else, I::I64Const(0), I::End]);
	}

	fn big_field(&self, func: &mut Function, local: u32, field_index: u32) {
		Self::emit_list(
			func,
			&[I::LocalGet(local), I::StructGet { struct_type_index: self.type_manager.big_int_type, field_index }],
		);
	}

	fn negative(&self, func: &mut Function, local: u32) {
		self.big_field(func, local, 0);
	}

	fn magnitude(&self, func: &mut Function, local: u32) {
		self.big_field(func, local, 1);
	}

	/// `local += delta` for an i32 local
	fn increment(func: &mut Function, local: u32, delta: i32) {
		Self::emit_list(func, &[I::LocalGet(local), I::I32Const(delta), I::I32Add, I::LocalSet(local)]);
	}

	/// `loop { if counter >= bound break; body; counter++ }` with i32 locals
	fn count_up(func: &mut Function, counter: u32, bound: u32, body: impl FnOnce(&mut Function)) {
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::Block(BlockType::Empty),
				I::Loop(BlockType::Empty),
					I::LocalGet(counter), I::LocalGet(bound), I::I32GeU, I::BrIf(1),
		]);
		body(func);
		Self::increment(func, counter, 1);
		Self::emit_list(func, &[I::Br(0), I::End, I::End]);
	}

	/// `loop { if counter == 0 break; counter--; body }` with an i32 local
	fn count_down(func: &mut Function, counter: u32, body: impl FnOnce(&mut Function)) {
		Self::emit_list(
			func,
			&[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(counter), I::I32Eqz, I::BrIf(1)],
		);
		Self::increment(func, counter, -1);
		body(func);
		Self::emit_list(func, &[I::Br(0), I::End, I::End]);
	}

	pub(crate) fn emit_int_runtime(&mut self) {
		if !self.int_runtime() {
			return;
		}
		self.emit_magnitude_functions();
		self.emit_signed_functions();
		self.emit_handle_functions();
		self.emit_int_functions();
		self.emit_exact_functions();
		self.emit_float_as_exact();
	}
}

/// The loop counters of `body` that stay in 0..i32::MAX: the item counters of for loops over lists (for_loop.rs
/// INDEX_SUFFIX), only ever set to 0 and stepped by ++ (the loop stops below the item count, which is an i32)
pub(in crate::wasm_emitter) fn bounded_counters(body: &Node) -> HashSet<String> {
	let (mut counters, mut other_writes) = (HashSet::new(), HashSet::new());
	body.visit(&mut |part| {
		if let Node::Key(target, op, value) = part {
			let Node::Symbol(name) = target.drop_meta() else { return };
			if !name.ends_with(crate::for_loop::INDEX_SUFFIX) {
				return;
			}
			let counts = match op {
				Op::Assign => matches!(value.drop_meta(), Node::Number(Number::Int(0))),
				Op::Inc => true,
				_ => !(op.is_compound_assign() || matches!(op, Op::Define | Op::Dec)),
			};
			if counts {
				counters.insert(name.clone());
			} else {
				other_writes.insert(name.clone());
			}
		}
	});
	counters.retain(|name| !other_writes.contains(name));
	counters
}
