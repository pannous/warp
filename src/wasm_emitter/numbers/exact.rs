//! Exact numbers by default: `1/3*3 == 1`, `1/3 + 1/3` is 2/3 (DESIGN.md "Exact numbers by default").
//!
//! An exact number is an Int (see big_int.rs) whose handle may also point at a `$Ratio` in the
//! same heap: numerator and denominator are Int payloads, the denominator is > 1 and coprime to
//! the numerator. A ratio is never integral and integers are never ratios, so integers keep the
//! i64 fast path and only the slow paths ask `is_ratio`. The `$Ratio` doubles as the payload of
//! an Int node, read back as `Number::Quotient`.
//!
//! Decimal literals are exact (`.1 + .2 == .3`, decision exact-decimals); floats come only from float sources (sin,
//! host values, `as float`, e-notation) and mixing one into an expression makes the result an f64.
//! Small operands (inline ratios, Ints within ±2^31) add, subtract, multiply and (in calculating programs) compare in i64
//! Division by zero yields the extended rationals' ±∞ = ±1/0 and NaN = 0/0 (never a quiet f64 NaN):
//! ∞ + 1 = ∞, 1/∞ = 0, ∞ - ∞ = NaN, NaN equals only NaN. Truncating ∞ or NaN to an integer traps.

use super::super::{WasmGcEmitter, NEW_TEXT_OF, TEXT_PIECE};
use num_bigint::BigInt;
use num_traits::One;
use wasm_encoder::*;
use Instruction as I;

/// Largest shift count: `1 << n` allocates n bits, a bigger count is a runtime error instead of an out-of-memory
pub const MAX_SHIFT_COUNT: i64 = 1 << 16;
/// exact_text(x): the text of any exact number, as the host prints it (`2.5`, `1/3`, `-∞`, big integers in full)
pub const EXACT_TEXT: &str = "exact_text";
/// decimal_text(x), `x as decimal`: an exact ratio in decimal digits, each repeating one overlined: 1/6 is 0.16̅
/// (card repeating-decimals); a period longer than 1000 digits stays the ratio's text (lib/prelude.warp writes it)
pub const DECIMAL_TEXT: &str = "decimal_text";
pub const DECIMAL_WORD: &str = "decimal";
/// lib/prelude.warp's decimal_text, by its prelude name; the program loads it where it writes `as decimal` (implied_words)
const DECIMAL_TEXT_WORD: &str = "prelude·decimal_text";
const DECIMAL_TEXT_NAME: &str = "decimal_text";
/// `a % b` and `a // b` of two Ints beyond the fixnum fast path (big_int.rs emit_int_op)
pub const INT_MOD: &str = "int_mod";
pub const INT_EUCLID_DIV: &str = "int_euclid_div";
/// Makes every ratio of a module: without it a module holds none
pub const RATIO_NEW: &str = "ratio_new";
/// The bits int_and, int_or and int_xor combine per step of an Int beyond the fixnums
const BIT_CHUNK: i64 = 32;
const BIT_CHUNK_MASK: i64 = (1 << BIT_CHUNK) - 1;
const INT_TEXT: &str = "int_text";
/// int_gcd(a, b): every ratio is normalized by one
pub(crate) const INT_GCD: &str = "int_gcd";
const DECIMAL_BASE: i64 = 10;
/// int_text divides a big integer by 10^18, a fixnum, and writes each remainder's 18 digits in i64 arithmetic
const CHUNK: i64 = 1_000_000_000_000_000_000;
/// A limb of a $BigInt (base 2^32) has fewer decimal digits than this
const LIMB_DIGITS: i32 = 10;
/// The field of a $BigInt holding its limbs (type_manager.rs)
const BIG_INT_LIMBS: u32 = 1;
/// exact_dispatch's shortcut takes operands within ±SMALL_LIMIT (inline ratio parts are smaller) as i64 parts
const SMALL_LIMIT: i64 = 1 << 31;
/// The shortcut's locals after the two operands: a.n, a.d, b.n, b.d (reused as gcd and rest), then the result's parts
const SMALL_PARTS: [u32; 4] = [2, 3, 4, 5];
const SMALL_NUMERATOR: u32 = 6;
const SMALL_DENOMINATOR: u32 = 7;
const SMALL_LOCALS: usize = 6;
/// A ratio whose denominator has no other prime factors is a terminating decimal
const DECIMAL_PRIMES: [i64; 2] = [2, 5];

/// The prelude word that writes `x as decimal`, for modules IMPLIED_WORDS: the cast calls it, the program does not
pub(crate) fn implied_word(node: &crate::node::Node) -> Option<&'static str> {
	node.symbol_name().is_some_and(|name| name.eq_ignore_ascii_case(DECIMAL_WORD)).then_some(DECIMAL_TEXT_NAME)
}

impl WasmGcEmitter {
	/// Decimal literal as the exact value of its shortest round-trip digits: `0.1` → 1/10
	pub(crate) fn emit_decimal_literal(&mut self, func: &mut Function, value: f64) {
		let (numerator, denominator) = decimal_fraction(value);
		self.emit_exact_literal(func, &numerator, &denominator);
	}

	pub(crate) fn emit_exact_literal(&mut self, func: &mut Function, numerator: &BigInt, denominator: &BigInt) {
		match denominator.is_one() {
			true => self.emit_int_literal(func, numerator),
			false => self.emit_number_constant(func, numerator, denominator),
		}
	}

	/// A literal beyond the fixnums (a big Int, a ratio) from a global set on its first evaluation: built at every
	/// evaluation, each one added a $BigInt or $Ratio to the handle heap, which keeps them until the run ends
	pub(in crate::wasm_emitter) fn emit_number_constant(
		&mut self,
		func: &mut Function,
		numerator: &BigInt,
		denominator: &BigInt,
	) {
		let key = (numerator.clone(), denominator.clone());
		let global = match self.number_constants.get(&key) {
			Some(global) => *global,
			None => {
				let global = self.declare_global_of(ValType::I64, ConstExpr::i64_const(0)); // 0 is never a handle
				let name = if denominator.is_one() {
					format!("number {numerator}")
				} else {
					format!("number {numerator}/{denominator}")
				};
				self.extra_global_names.push((global, name));
				self.number_constants.insert(key, global);
				global
			}
		};
		Self::emit_list(func, &[I::GlobalGet(global), I::I64Eqz, I::If(BlockType::Empty)]);
		self.emit_built_int_literal(func, numerator);
		if !denominator.is_one() {
			self.emit_built_int_literal(func, denominator);
			self.emit_call(func, "exact_div");
		}
		Self::emit_list(func, &[I::GlobalSet(global), I::End, I::GlobalGet(global)]);
	}

	fn ratio_ref(&self) -> HeapType {
		HeapType::Concrete(self.type_manager.ratio_type)
	}

	/// Push `numerator(x) * denominator(y)`
	fn cross_product(&self, f: &mut Function, x: u32, y: u32) {
		self.part_product(f, x, "exact_numerator", y, "exact_denominator");
	}

	fn part_product(&self, f: &mut Function, x: u32, x_part: &str, y: u32, y_part: &str) {
		f.instruction(&I::LocalGet(x));
		self.call(f, x_part);
		f.instruction(&I::LocalGet(y));
		self.call(f, y_part);
		self.call(f, "int_mul");
	}

	/// Push i32: is either local a ratio
	fn any_ratio(&self, f: &mut Function, a: u32, b: u32) {
		f.instruction(&I::LocalGet(a));
		self.call(f, "is_ratio");
		f.instruction(&I::LocalGet(b));
		self.call(f, "is_ratio");
		f.instruction(&I::I32Or);
	}

	/// Push i32: Int in `local` < 0
	fn is_negative(&self, f: &mut Function, local: u32) {
		Self::emit_list(f, &[I::LocalGet(local), I::I64Const(0)]);
		self.call(f, "int_cmp");
		Self::emit_list(f, &[I::I32Const(0), I::I32LtS]);
	}

	/// `local = -local` for an Int
	fn negate_local(&self, f: &mut Function, local: u32) {
		Self::emit_list(f, &[I::I64Const(0), I::LocalGet(local)]);
		self.call(f, "int_sub");
		f.instruction(&I::LocalSet(local));
	}

	/// Return `fixnum_result` at once when a and b (params 0, 1) are fixnums and b is not 0: a division or remainder of
	/// two machine-sized Ints needs neither a $BigInt nor a $Ratio, which int_store keeps for the whole run
	fn fixnum_division_shortcut(&self, f: &mut Function, fixnum_result: &[Instruction]) {
		self.emit_fixnum_test(f, &[0, 1]);
		Self::emit_list(f, &[I::LocalGet(1), I::I64Const(0), I::I64Ne, I::I32And, I::If(BlockType::Empty)]);
		Self::emit_list(f, fixnum_result);
		Self::emit_list(f, &[I::Return, I::End]);
	}

	/// Push i32 1 when operand `x` is small, an inline ratio or an Int within ±2^31, after loading its numerator and
	/// denominator into `numerator`, `denominator`: products of two such parts stay far inside the fixnums
	fn small_parts(&self, f: &mut Function, x: u32, numerator: u32, denominator: u32) {
		self.emit_inline_ratio_test(f, x);
		f.instruction(&I::If(BlockType::Result(ValType::I32)));
		self.emit_inline_numerator(f, x);
		f.instruction(&I::LocalSet(numerator));
		self.emit_inline_denominator(f, x);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalSet(denominator),
			I::I32Const(1),
			I::Else,
				I::LocalGet(x), I::LocalSet(numerator),
				I::I64Const(1), I::LocalSet(denominator),
				I::LocalGet(x), I::I64Const(SMALL_LIMIT), I::I64Add, I::I64Const(2 * SMALL_LIMIT), I::I64LtU,
			I::End,
		]);
	}

	/// Both operands small (small_parts into SMALL_PARTS): run `small_op` on their parts in i64 and return
	fn small_shortcut(&self, f: &mut Function, small_op: impl FnOnce(&Self, &mut Function)) {
		let [a_numerator, a_denominator, b_numerator, b_denominator] = SMALL_PARTS;
		self.small_parts(f, 0, a_numerator, a_denominator);
		self.small_parts(f, 1, b_numerator, b_denominator);
		Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty)]);
		small_op(self, f);
		Self::emit_list(f, &[I::Return, I::End]);
	}

	/// The ratio numerator/denominator of `combine`'s two i64 results, normalized by an i64 Euclid gcd (the denominator
	/// is positive): an Int, an inline ratio, or ratio_new's
	fn small_ratio(&self, f: &mut Function, combine: &[Instruction]) {
		let [gcd, rest, ..] = SMALL_PARTS;
		Self::emit_list(f, combine);
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalSet(SMALL_DENOMINATOR),
			I::LocalTee(SMALL_NUMERATOR), I::LocalGet(SMALL_NUMERATOR), I::I64Const(63), I::I64ShrS, I::I64Xor,
			I::LocalGet(SMALL_NUMERATOR), I::I64Const(63), I::I64ShrS, I::I64Sub, I::LocalSet(gcd),
			I::LocalGet(SMALL_DENOMINATOR), I::LocalSet(rest),
			I::Block(BlockType::Empty),
				I::Loop(BlockType::Empty),
					I::LocalGet(rest), I::I64Eqz, I::BrIf(1),
					I::LocalGet(gcd), I::LocalGet(rest), I::I64RemU, I::LocalGet(rest), I::LocalSet(gcd),
					I::LocalSet(rest),
					I::Br(0),
			I::End, I::End,
			I::LocalGet(SMALL_NUMERATOR), I::LocalGet(gcd), I::I64DivS, I::LocalSet(SMALL_NUMERATOR),
			I::LocalGet(SMALL_DENOMINATOR), I::LocalGet(gcd), I::I64DivU, I::LocalTee(SMALL_DENOMINATOR),
			I::I64Const(1), I::I64Eq, I::If(BlockType::Result(ValType::I64)),
				I::LocalGet(SMALL_NUMERATOR),
			I::Else,
		]);
		self.emit_inline_ratio_fits(f, SMALL_NUMERATOR, SMALL_DENOMINATOR);
		f.instruction(&I::If(BlockType::Result(ValType::I64)));
		self.emit_inline_ratio(f, SMALL_NUMERATOR, SMALL_DENOMINATOR);
		Self::emit_list(f, &[I::Else, I::LocalGet(SMALL_NUMERATOR), I::LocalGet(SMALL_DENOMINATOR)]);
		self.call(f, RATIO_NEW);
		Self::emit_list(f, &[I::End, I::End]);
	}

	/// `name(a, b)`: `shortcut`, then ratio_op when an operand is a ratio, else the integer slow path
	fn exact_dispatch(
		&mut self,
		name: &'static str,
		result: ValType,
		shortcut: impl FnOnce(&Self, &mut Function),
		ratio_op: impl FnOnce(&Self, &mut Function),
		integer_op: &str,
	) {
		let i64t = ValType::I64;
		self.runtime_function(name, vec![i64t, i64t], vec![result], vec![i64t; SMALL_LOCALS], |s, f| {
			shortcut(s, f);
			s.any_ratio(f, 0, 1);
			f.instruction(&I::If(BlockType::Result(result)));
			ratio_op(s, f);
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, integer_op);
			f.instruction(&I::End);
		});
	}

	/// `name(x)`: ratio_op on a ratio, else the integer slow path
	fn exact_unary(
		&mut self,
		name: &'static str,
		result: ValType,
		ratio_op: impl FnOnce(&Self, &mut Function),
		integer_op: &str,
	) {
		self.runtime_function(name, vec![ValType::I64], vec![result], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "is_ratio");
			f.instruction(&I::If(BlockType::Result(result)));
			ratio_op(s, f);
			Self::emit_list(f, &[I::Else, I::LocalGet(0)]);
			s.call(f, integer_op);
			f.instruction(&I::End);
		});
	}

	pub(crate) fn emit_exact_functions(&mut self) {
		self.emit_exact_integers();
		self.emit_exact_ratios();
		self.emit_exact_operations();
	}

	/// The Int helpers with inline fast paths: add, sub, quot, rem, gcd
	fn emit_exact_integers(&mut self) {
		let i64t = ValType::I64;

		for (name, machine, slow) in [("int_add", I::I64Add, "int_add_slow"), ("int_sub", I::I64Sub, "int_sub_slow")] {
			// locals: result
			self.runtime_function(name, vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1), machine, I::LocalSet(2)]);
				s.emit_fixnum_test(f, &[0, 1, 2]);
				Self::emit_list(
					f,
					&[I::If(BlockType::Result(i64t)), I::LocalGet(2), I::Else, I::LocalGet(0), I::LocalGet(1)],
				);
				s.call(f, slow);
				f.instruction(&I::End);
			});
		}

		// int_quot / int_rem: truncated like i64.div_s / i64.rem_s, divisor nonzero
		for (name, machine, slow) in
			[("int_quot", I::I64DivS, "int_quot_slow"), ("int_rem", I::I64RemS, "int_rem_slow")]
		{
			self.runtime_function(name, vec![i64t, i64t], vec![i64t], vec![], |s, f| {
				s.emit_fixnum_test(f, &[0, 1]);
				Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(0), I::LocalGet(1), machine]);
				s.call(f, "int_from_i64"); // 2^62 / -1 leaves the fixnum range
				Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, slow);
				f.instruction(&I::End);
			});
		}

		// int_mod(a, b), int_euclid_div(a, b): `%` and `//` of two Ints, Euclidean (0 ≤ r < |b|, a = b·q + r), from the
		// truncated int_rem / int_quot without the ratio runtime (card int-i64): a negative remainder takes |b| more, its
		// quotient one step away from b's sign. locals: the result, the remainder
		for (name, truncated, step, for_negative_b, for_positive_b) in [
			(INT_MOD, "int_rem", I::LocalGet(1), "int_sub", "int_add"),
			(INT_EUCLID_DIV, "int_quot", I::I64Const(1), "int_add", "int_sub"),
		] {
			self.runtime_function(name, vec![i64t, i64t], vec![i64t], vec![i64t, i64t], |s, f| {
				let (result, remainder) = (2, 3);
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, truncated);
				Self::emit_list(f, &[I::LocalSet(result), I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, "int_rem");
				f.instruction(&I::LocalSet(remainder));
				s.is_negative(f, remainder);
				f.instruction(&I::If(BlockType::Empty));
				s.is_negative(f, 1);
				Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(result), step.clone()]);
				s.call(f, for_negative_b);
				Self::emit_list(f, &[I::Else, I::LocalGet(result), step]);
				s.call(f, for_positive_b);
				Self::emit_list(f, &[I::End, I::LocalSet(result), I::End, I::LocalGet(result)]);
			});
		}

		// int_gcd(a, b) >= 0 by Euclid; locals: remainder
		self.runtime_function(INT_GCD, vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			for local in [0, 1] {
				s.is_negative(f, local);
				f.instruction(&I::If(BlockType::Empty));
				s.negate_local(f, local);
				f.instruction(&I::End);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(1), I::I64Eqz, I::BrIf(1),
						I::LocalGet(0), I::LocalGet(1),
			]);
			s.call(f, "int_rem");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(2),
				I::LocalGet(1), I::LocalSet(0),
				I::LocalGet(2), I::LocalSet(1),
				I::Br(0),
				I::End,
				I::End,
				I::LocalGet(0),
			]);
		});
	}

	/// The ratios: is_ratio, numerator, denominator, ratio_new, exact_div
	fn emit_exact_ratios(&mut self) {
		let (i64t, i32t) = (ValType::I64, ValType::I32);
		let ratio = self.type_manager.ratio_type;
		let ratio_ref = self.ratio_ref();

		self.runtime_function("is_ratio", vec![i64t], vec![i32t], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0]);
			Self::emit_list(f, &[I::If(BlockType::Result(i32t)), I::I32Const(0), I::Else]);
			s.emit_inline_ratio_test(f, 0);
			Self::emit_list(f, &[I::If(BlockType::Result(i32t)), I::I32Const(1), I::Else]);
			s.emit_heap_get(f, 0);
			Self::emit_list(f, &[I::RefTestNonNull(ratio_ref), I::End, I::End]);
		});

		for (name, field, integer_value) in [("exact_numerator", 0, None), ("exact_denominator", 1, Some(1))] {
			self.runtime_function(name, vec![i64t], vec![i64t], vec![], |s, f| {
				f.instruction(&I::LocalGet(0));
				s.call(f, "is_ratio");
				f.instruction(&I::If(BlockType::Result(i64t)));
				s.emit_inline_ratio_test(f, 0);
				f.instruction(&I::If(BlockType::Result(i64t)));
				match integer_value {
					Some(_) => s.emit_inline_denominator(f, 0),
					None => s.emit_inline_numerator(f, 0),
				}
				f.instruction(&I::Else);
				s.emit_heap_get(f, 0);
				Self::emit_list(
					f,
					&[I::RefCastNonNull(ratio_ref), I::StructGet { struct_type_index: ratio, field_index: field }],
				);
				s.call(f, "int_from_payload");
				Self::emit_list(
					f,
					&[
						I::End,
						I::Else,
						match integer_value {
							Some(one) => I::I64Const(one),
							None => I::LocalGet(0),
						},
						I::End,
					],
				);
			});
		}

		// ratio_new(n, d): normalized n/d for integers, d nonzero; locals: gcd
		self.runtime_function(RATIO_NEW, vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			s.is_negative(f, 1);
			f.instruction(&I::If(BlockType::Empty));
			s.negate_local(f, 0);
			s.negate_local(f, 1);
			Self::emit_list(f, &[I::End, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, INT_GCD);
			// gcd 0 only for 0/0 (NaN); compared as an Int: a big gcd's handle is negative (card exact-decimals)
			Self::emit_list(f, &[I::LocalTee(2), I::I64Const(1)]);
			s.call(f, "int_cmp");
			Self::emit_list(f, &[I::I32Const(0), I::I32GtS, I::If(BlockType::Empty)]);
			for local in [0, 1] {
				Self::emit_list(f, &[I::LocalGet(local), I::LocalGet(2)]);
				s.call(f, "int_quot");
				f.instruction(&I::LocalSet(local));
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::End,
				I::LocalGet(1), I::I64Const(1), I::I64Eq, I::If(BlockType::Result(i64t)),
					I::LocalGet(0),
				I::Else,
			]);
			s.emit_inline_ratio_fits(f, 0, 1);
			f.instruction(&I::If(BlockType::Result(i64t)));
			s.emit_inline_ratio(f, 0, 1);
			f.instruction(&I::Else);
			s.emit_int_payload(f, 0);
			s.emit_int_payload(f, 1);
			f.instruction(&I::StructNew(ratio));
			s.call(f, "int_store");
			Self::emit_list(f, &[I::End, I::End]);
		});

		// exact_div(a, b): a/b, an integer whenever b divides a; locals: quotient, remainder
		let (quotient, remainder) = (2, 3);
		self.runtime_function("exact_div", vec![i64t, i64t], vec![i64t], vec![i64t, i64t], |s, f| {
			let ratio_of_both = |f: &mut Function| {
				s.cross_product(f, 0, 1);
				s.cross_product(f, 1, 0);
				s.call(f, RATIO_NEW);
			};
			// P66: an exact division by zero is the catchable error divide_by_zero (0 is the fixnum 0)
			Self::emit_list(f, &[I::LocalGet(1), I::I64Eqz]);
			s.emit_fail_if(f, super::super::lists::list_ops::DIVIDE_BY_ZERO);
			s.emit_fixnum_test(f, &[0, 1]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::If(BlockType::Result(i64t)),
					I::LocalGet(1), I::I64Eqz, I::If(BlockType::Result(i32t)),
						I::I32Const(0),
					I::Else,
						I::LocalGet(0), I::LocalGet(1), I::I64RemS, I::I64Eqz,
					I::End,
					I::If(BlockType::Result(i64t)),
						I::LocalGet(0), I::LocalGet(1), I::I64DivS,
			]);
			s.call(f, "int_from_i64");
			f.instruction(&I::Else);
			ratio_of_both(f);
			Self::emit_list(f, &[I::End, I::Else]);
			// big integers: one long division gives quotient and remainder; ratio_new's gcd only when b does not divide a
			for operand in [0, 1] {
				f.instruction(&I::LocalGet(operand));
				s.call(f, "is_ratio");
			}
			Self::emit_list(f, &[I::I32Or, I::LocalGet(1), I::I64Eqz, I::I32Or, I::If(BlockType::Result(i64t))]);
			ratio_of_both(f);
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "int_divmod_slow");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(remainder),
				I::LocalSet(quotient),
				I::LocalGet(remainder), I::I64Eqz, I::If(BlockType::Result(i64t)),
					I::LocalGet(quotient),
				I::Else,
			]);
			ratio_of_both(f);
			Self::emit_list(f, &[I::End, I::End, I::End]);
		});
	}

	/// The exact operations the Int slow paths dispatch to: add, sub, compare, trunc, rem, mod, floor, pow, shifts
	fn emit_exact_operations(&mut self) {
		let (i64t, i32t, f64t) = (ValType::I64, ValType::I32, ValType::F64);

		let [a_numerator, a_denominator, b_numerator, b_denominator] = SMALL_PARTS;
		let denominators = [I::LocalGet(a_denominator), I::LocalGet(b_denominator), I::I64Mul];
		for (name, combine, small_combine, integer_op) in
			[("exact_add", "int_add", I::I64Add, "int_add_slow"), ("exact_sub", "int_sub", I::I64Sub, "int_sub_slow")]
		{
			let small_sum = [
				I::LocalGet(a_numerator),
				I::LocalGet(b_denominator),
				I::I64Mul,
				I::LocalGet(b_numerator),
				I::LocalGet(a_denominator),
				I::I64Mul,
				small_combine,
				denominators[0].clone(),
				denominators[1].clone(),
				denominators[2].clone(),
			];
			self.exact_dispatch(
				name,
				i64t,
				|s, f| s.small_shortcut(f, |s, f| s.small_ratio(f, &small_sum)),
				|s, f| {
					s.cross_product(f, 0, 1);
					s.cross_product(f, 1, 0);
					s.call(f, combine);
					s.part_product(f, 0, "exact_denominator", 1, "exact_denominator");
					s.call(f, RATIO_NEW);
				},
				integer_op,
			);
		}
		let small_product = [
			I::LocalGet(a_numerator),
			I::LocalGet(b_numerator),
			I::I64Mul,
			denominators[0].clone(),
			denominators[1].clone(),
			denominators[2].clone(),
		];
		self.exact_dispatch(
			"exact_mul",
			i64t,
			|s, f| s.small_shortcut(f, |s, f| s.small_ratio(f, &small_product)),
			|s, f| {
				s.part_product(f, 0, "exact_numerator", 1, "exact_numerator");
				s.part_product(f, 0, "exact_denominator", 1, "exact_denominator");
				s.call(f, RATIO_NEW);
			},
			"int_mul_slow",
		);
		// exact_infinity_rank(x): -1 for -∞, 1 for ∞, 0 for finite x (and NaN)
		self.runtime_function("exact_infinity_rank", vec![i64t], vec![i64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "exact_denominator");
			Self::emit_list(f, &[I::I64Eqz, I::If(BlockType::Result(i64t)), I::LocalGet(0)]);
			s.call(f, "exact_numerator");
			Self::emit_list(f, &[I::Else, I::I64Const(0), I::End]);
		});
		self.runtime_function("exact_is_nan", vec![i64t], vec![i32t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "exact_denominator");
			f.instruction(&I::LocalGet(0));
			s.call(f, "exact_numerator");
			Self::emit_list(f, &[I::I64Or, I::I64Eqz]);
		});
		// -1/0/1, and 2 (unequal) when exactly one side is NaN.
		// Finite denominators are positive, so a/b <=> c/d is a*d <=> c*b
		// small: the sign of a.n·b.d - b.n·a.d. exact_cmp ships in every module (equality) and a hello-world site has a
		// size budget, so the shortcut comes only with exact arithmetic
		#[rustfmt::skip]
		let small_sign = [
			I::LocalGet(a_numerator), I::LocalGet(b_denominator), I::I64Mul, I::LocalGet(b_numerator),
			I::LocalGet(a_denominator), I::I64Mul, I::I64Sub, I::LocalTee(SMALL_NUMERATOR), I::I64Const(0), I::I64GtS,
			I::LocalGet(SMALL_NUMERATOR), I::I64Const(0), I::I64LtS, I::I32Sub,
		];
		let calculates = self.should_emit_function(super::super::EXACT_ARITHMETIC);
		self.exact_dispatch(
			"exact_cmp",
			i32t,
			|s, f| {
				if calculates {
					s.small_shortcut(f, |_, f| Self::emit_list(f, &small_sign))
				}
			},
			|s, f| {
				let is_nan = |f: &mut Function, local: u32| {
					f.instruction(&I::LocalGet(local));
					s.call(f, "exact_is_nan");
				};
				is_nan(f, 0);
				is_nan(f, 1);
				Self::emit_list(f, &[I::I32Or, I::If(BlockType::Result(i32t))]);
				is_nan(f, 0);
				is_nan(f, 1);
				Self::emit_list(f, &[I::I32Ne, I::I32Const(1), I::I32Shl, I::Else]);
				s.part_product(f, 0, "exact_denominator", 1, "exact_denominator");
				Self::emit_list(f, &[I::I64Eqz, I::If(BlockType::Result(i32t))]);
				for x in [0, 1] {
					f.instruction(&I::LocalGet(x));
					s.call(f, "exact_infinity_rank");
				}
				s.call(f, "int_cmp");
				f.instruction(&I::Else);
				s.cross_product(f, 0, 1);
				s.cross_product(f, 1, 0);
				s.call(f, "int_cmp");
				Self::emit_list(f, &[I::End, I::End]);
			},
			"int_cmp",
		);

		self.exact_unary(
			"exact_neg",
			i64t,
			|s, f| {
				Self::emit_list(f, &[I::I64Const(0), I::LocalGet(0)]);
				s.call(f, "exact_numerator");
				s.call(f, "int_sub");
				f.instruction(&I::LocalGet(0));
				s.call(f, "exact_denominator");
				s.call(f, RATIO_NEW);
			},
			"int_neg_slow",
		);
		self.exact_unary(
			"exact_abs",
			i64t,
			|s, f| {
				Self::emit_list(f, &[I::LocalGet(0), I::I64Const(0)]);
				s.call(f, "exact_cmp");
				Self::emit_list(f, &[I::I32Const(0), I::I32LtS, I::If(BlockType::Result(i64t)), I::LocalGet(0)]);
				s.call(f, "exact_neg");
				Self::emit_list(f, &[I::Else, I::LocalGet(0), I::End]);
			},
			"int_abs_slow",
		);
		self.exact_unary(
			"exact_to_f64",
			f64t,
			|s, f| {
				f.instruction(&I::LocalGet(0));
				s.call(f, "exact_numerator");
				s.call(f, "int_to_f64");
				f.instruction(&I::LocalGet(0));
				s.call(f, "exact_denominator");
				s.call(f, "int_to_f64");
				f.instruction(&I::F64Div);
			},
			"int_to_f64",
		);

		// exact_trunc(x): integer part, rounded toward zero
		self.runtime_function("exact_trunc", vec![i64t], vec![i64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "is_ratio");
			f.instruction(&I::If(BlockType::Result(i64t)));
			s.part_quotient(f, 0);
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::End]);
		});

		self.runtime_function("exact_to_i64_wrap", vec![i64t], vec![i64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "exact_trunc");
			s.call(f, "int_to_i64_wrap");
		});

		// exact_rem(a, b) = a - b*trunc(a/b), the sign of a like i64.rem_s
		self.exact_dispatch(
			"exact_rem",
			i64t,
			|s, f| s.fixnum_division_shortcut(f, &[I::LocalGet(0), I::LocalGet(1), I::I64RemS]),
			|s, f| {
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1), I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, "exact_div");
				s.call(f, "exact_trunc");
				s.call(f, "exact_mul");
				s.call(f, "exact_sub");
			},
			"int_rem_slow",
		);

		// exact_mod(a, b): Euclidean remainder, 0 ≤ r < |b|: the truncated remainder plus |b| when negative; locals: r
		let euclidean_remainder = super::super::numbers::big_int::euclidean_remainder(0, 1, 2);
		self.runtime_function("exact_mod", vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			s.fixnum_division_shortcut(f, &euclidean_remainder);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "exact_rem");
			Self::emit_list(f, &[I::LocalTee(2), I::I64Const(0)]);
			s.call(f, "exact_cmp");
			Self::emit_list(
				f,
				&[I::I32Const(0), I::I32LtS, I::If(BlockType::Result(i64t)), I::LocalGet(2), I::LocalGet(1)],
			);
			s.call(f, "exact_abs");
			s.call(f, "exact_add");
			Self::emit_list(f, &[I::Else, I::LocalGet(2), I::End]);
		});

		// exact_floor(x): the largest integer ≤ x, exact also beyond the f64 range (`floor(10^30/3)`, `a//b`)
		self.runtime_function("exact_floor", vec![i64t], vec![i64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "is_ratio");
			Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(0)]);
			s.call(f, "exact_trunc");
			Self::emit_list(f, &[I::LocalGet(0), I::I64Const(0)]);
			s.call(f, "exact_cmp");
			// a negative ratio truncates up: one less
			Self::emit_list(f, &[I::I32Const(0), I::I32LtS, I::I64ExtendI32U]);
			s.call(f, "exact_sub");
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::End]);
		});

		// exact_euclid_div(a, b): the quotient of a // b that goes with a % b: floor(a/b) for b > 0, ceil(a/b) =
		// -floor(-a/b) for b < 0
		// for fixnums (a - a % b) / b, which divides exactly
		let euclidean_quotient =
			[&[I::LocalGet(0)][..], &euclidean_remainder, &[I::I64Sub, I::LocalGet(1), I::I64DivS]].concat();
		self.runtime_function("exact_euclid_div", vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			s.fixnum_division_shortcut(f, &euclidean_quotient);
			Self::emit_list(f, &[I::LocalGet(1), I::I64Const(0)]);
			s.call(f, "exact_cmp");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Const(0), I::I32LtS, I::If(BlockType::Result(i64t)),
					I::I64Const(0), I::I64Const(0), I::LocalGet(0), I::LocalGet(1),
			]);
			s.call(f, "exact_div");
			s.call(f, "exact_sub");
			s.call(f, "exact_floor");
			s.call(f, "exact_sub");
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "exact_div");
			s.call(f, "exact_floor");
			f.instruction(&I::End);
		});

		// x /= y: an integer x stays an integer, the Euclidean quotient (x - x % y) / y matching %; a ratio x divides exactly
		self.runtime_function("exact_div_assign", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "is_ratio");
			Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "exact_div");
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "exact_mod");
			s.call(f, "exact_sub");
			f.instruction(&I::LocalGet(1));
			s.call(f, "exact_div");
			f.instruction(&I::End);
		});

		// exact_pow(base, exponent): integer exponents only, a negative one inverts: 2^-1 = 1/2
		self.runtime_function("exact_pow", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			f.instruction(&I::LocalGet(1));
			s.call(f, "is_ratio");
			Self::emit_list(f, &[I::If(BlockType::Empty), I::Unreachable, I::End]);
			s.is_negative(f, 1);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::I64Const(1), I::LocalGet(0)]);
			s.call(f, "exact_div");
			f.instruction(&I::LocalSet(0));
			s.negate_local(f, 1);
			Self::emit_list(f, &[I::End, I::LocalGet(0)]);
			s.call(f, "is_ratio");
			f.instruction(&I::If(BlockType::Result(i64t)));
			for part in ["exact_numerator", "exact_denominator"] {
				f.instruction(&I::LocalGet(0));
				s.call(f, part);
				f.instruction(&I::LocalGet(1));
				s.call(f, "int_pow");
			}
			s.call(f, RATIO_NEW);
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "int_pow");
			f.instruction(&I::End);
		});

		// int_shift_left(a, n) = a * 2^n and int_shift_right(a, n) = floor(a / 2^n): arithmetic shifts of an unbounded Int
		self.runtime_function("int_shift_left", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			s.emit_shift_operand_checks(f);
			s.emit_power_of_two_shifted(f);
			s.call(f, "int_mul");
		});
		self.runtime_function("int_shift_right", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			s.emit_shift_operand_checks(f);
			s.emit_power_of_two_shifted(f);
			s.call(f, "exact_div_assign");
		});
		self.emit_bitwise_functions();
	}

	/// int_xor(a, b), int_and(a, b), int_or(a, b) on the whole two's complement Ints (card big-int): until both are
	/// fixnums, combine their low BIT_CHUNK bits and shift both right (floor, so a negative Int keeps its sign bits);
	/// the result is the sum of the combined chunks, each shifted back into place. Locals: result, shift
	fn emit_bitwise_functions(&mut self) {
		let i64t = ValType::I64;
		let (a, b, result, shift) = (0, 1, 2, 3);
		for (name, combine) in [("int_xor", I::I64Xor), ("int_and", I::I64And), ("int_or", I::I64Or)] {
			self.runtime_function(name, vec![i64t, i64t], vec![i64t], vec![i64t, i64t], |s, f| {
				let add_shifted = |s: &Self, f: &mut Function| {
					Self::emit_list(f, &[I::LocalGet(shift)]);
					s.call(f, "int_shift_left");
					f.instruction(&I::LocalGet(result));
					s.call(f, "exact_add");
					f.instruction(&I::LocalSet(result));
				};
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				s.emit_fixnum_test(f, &[a, b]);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(a), I::LocalGet(b), combine.clone()]);
				s.call(f, "int_from_i64");
				add_shifted(s, f);
				Self::emit_list(f, &[I::Br(2), I::End]);
				for local in [a, b] {
					f.instruction(&I::LocalGet(local));
					s.call(f, "int_to_i64_wrap");
				}
				Self::emit_list(f, &[combine.clone(), I::I64Const(BIT_CHUNK_MASK), I::I64And]);
				add_shifted(s, f);
				for local in [a, b] {
					Self::emit_list(f, &[I::LocalGet(local), I::I64Const(BIT_CHUNK)]);
					s.call(f, "int_shift_right");
					f.instruction(&I::LocalSet(local));
				}
				#[rustfmt::skip]
				Self::emit_list(f, &[I::LocalGet(shift), I::I64Const(BIT_CHUNK), I::I64Add, I::LocalSet(shift), I::Br(0), I::End, I::End, I::LocalGet(result)]);
			});
		}
	}

	/// Trap unless local 0 is an integer and local 1 a shift count in 0..=MAX_SHIFT_COUNT; each check runs only if the ones before it passed
	fn emit_shift_operand_checks(&self, f: &mut Function) {
		let trap_if = |f: &mut Function| Self::emit_list(f, &[I::If(BlockType::Empty), I::Unreachable, I::End]);
		for local in [0, 1] {
			f.instruction(&I::LocalGet(local));
			self.call(f, "is_ratio");
			trap_if(f);
		}
		self.is_negative(f, 1);
		trap_if(f);
		Self::emit_list(f, &[I::LocalGet(1), I::I64Const(MAX_SHIFT_COUNT)]);
		self.call(f, "int_cmp");
		Self::emit_list(f, &[I::I32Const(0), I::I32GtS]);
		trap_if(f);
	}

	/// Push local 0, then 2^local 1
	fn emit_power_of_two_shifted(&self, f: &mut Function) {
		Self::emit_list(f, &[I::LocalGet(0), I::I64Const(2), I::LocalGet(1)]);
		self.call(f, "exact_pow");
	}

	/// int_text(n) and exact_text(x): the decimal text of an integer of any size, and of any exact number
	pub(crate) fn emit_exact_text(&mut self) {
		if !self.should_emit_function(EXACT_TEXT) {
			return;
		}
		let (i64t, i32t, node_ref) = (ValType::I64, ValType::I32, ValType::Ref(self.node_ref(false)));
		let constant = |s: &mut Self, text: &str| s.string_table.literal(text);
		let [minus, dot, slash, empty, infinity, negative_infinity, not_a_number] =
			["-", ".", "/", "", "∞", "-∞", "NaN"].map(|text| constant(self, text));
		let push_text = |s: &Self, f: &mut Function, literal: (u32, u32)| {
			s.emit_constant_text(f, literal);
		};

		// int_text(n): sign, then the digits written from the last one back, a chunk of 18 per big division
		// (card out-fuel), into room for LIMB_DIGITS per limb; the text starts at the first digit written.
		// locals: negative, digits, bytes, length, chunk, last (the chunk is the value's top one)
		let locals = vec![i32t, i32t, self.text_ref_nullable(), i32t, i64t, i32t];
		let (big_int, text_type) = (self.type_manager.big_int_type, self.type_manager.text_type);
		self.runtime_function(INT_TEXT, vec![i64t], vec![node_ref], locals, |s, f| {
			let (value, negative, digits, bytes, length, chunk, last) = (0, 1, 2, 3, 4, 5, 6);
			s.is_negative(f, value);
			Self::emit_list(f, &[I::LocalTee(negative), I::If(BlockType::Empty)]);
			s.negate_local(f, value);
			Self::emit_list(f, &[I::End, I::LocalGet(value)]);
			s.call(f, "int_unbox");
			// room for the sign and the "0" of zero, which has no limbs
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::StructGet { struct_type_index: big_int, field_index: BIG_INT_LIMBS }, I::ArrayLen,
				I::I32Const(LIMB_DIGITS), I::I32Mul, I::I32Const(2), I::I32Add, I::LocalTee(digits), I::LocalTee(length),
				I::ArrayNewDefault(text_type), I::LocalSet(bytes),
			]);
			Self::emit_list(f, &[I::Loop(BlockType::Empty), I::LocalGet(value), I::I64Const(CHUNK)]);
			s.call(f, "int_cmp");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Const(0), I::I32LtS, I::LocalTee(last), I::If(BlockType::Empty),
					I::LocalGet(value), I::LocalSet(chunk),
				I::Else,
					I::LocalGet(value), I::I64Const(CHUNK),
			]);
			s.call(f, "int_rem");
			// a lower chunk plus 10^18: its digits with their zeros, then the leading 1 the loop stops at
			Self::emit_list(
				f,
				&[I::I64Const(CHUNK), I::I64Add, I::LocalSet(chunk), I::LocalGet(value), I::I64Const(CHUNK)],
			);
			s.call(f, "int_quot");
			#[rustfmt::skip]
			Self::emit_list(f, &[
					I::LocalSet(value),
				I::End,
				I::Loop(BlockType::Empty),
					I::LocalGet(bytes), I::LocalGet(digits), I::I32Const(1), I::I32Sub, I::LocalTee(digits),
					I::LocalGet(chunk), I::I64Const(DECIMAL_BASE), I::I64RemU, I::I32WrapI64,
					I::I32Const(b'0' as i32), I::I32Add, I::ArraySet(text_type),
					I::LocalGet(chunk), I::I64Const(DECIMAL_BASE), I::I64DivU, I::LocalTee(chunk),
					// the top chunk stops at 0, a lower one at its leading 1
					I::I64Const(1), I::LocalGet(last), I::I64ExtendI32U, I::I64Sub, I::I64GtU, I::BrIf(0),
				I::End,
				I::LocalGet(last), I::I32Eqz, I::BrIf(0),
				I::End,
				I::LocalGet(negative), I::If(BlockType::Empty),
					I::LocalGet(bytes), I::LocalGet(digits), I::I32Const(1), I::I32Sub, I::LocalTee(digits),
					I::I32Const(b'-' as i32), I::ArraySet(text_type),
				I::End,
			]);
			s.emit_text_piece(f, (bytes, digits, length));
		});

		// exact_text(x); locals: numerator, denominator, rest, scale, fraction digits
		let nullable = ValType::Ref(self.node_ref(true));
		self.runtime_function(
			EXACT_TEXT,
			vec![i64t],
			vec![node_ref],
			vec![i64t, i64t, i64t, i64t, nullable],
			|s, f| {
				let (value, numerator, denominator, rest, scale, fraction) = (0, 1, 2, 3, 4, 5);
				let concat = |f: &mut Function| s.call(f, crate::wasm_emitter::text::text_builtins::TEXT_CONCAT);
				f.instruction(&I::LocalGet(value));
				s.call(f, "is_ratio");
				Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(value)]);
				s.call(f, INT_TEXT);
				Self::emit_list(f, &[I::Return, I::End, I::LocalGet(value)]);
				s.call(f, "exact_numerator");
				Self::emit_list(f, &[I::LocalSet(numerator), I::LocalGet(value)]);
				s.call(f, "exact_denominator");
				// ±∞ = ±1/0 and NaN = 0/0
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalTee(denominator), I::I64Eqz, I::If(BlockType::Empty),
						I::LocalGet(numerator), I::I64Eqz, I::If(BlockType::Result(node_ref)),
				]);
				push_text(s, f, not_a_number);
				Self::emit_list(
					f,
					&[I::Else, I::LocalGet(numerator), I::I64Const(0), I::I64GtS, I::If(BlockType::Result(node_ref))],
				);
				push_text(s, f, infinity);
				f.instruction(&I::Else);
				push_text(s, f, negative_infinity);
				Self::emit_list(f, &[I::End, I::End, I::Return, I::End]);
				// a denominator with a prime other than 2 and 5 never terminates: numerator/denominator
				Self::emit_list(f, &[I::LocalGet(denominator), I::LocalSet(rest)]);
				for prime in DECIMAL_PRIMES {
					s.divide_out(f, rest, prime, &[]);
				}
				Self::emit_list(
					f,
					&[I::LocalGet(rest), I::I64Const(1), I::I64Ne, I::If(BlockType::Empty), I::LocalGet(numerator)],
				);
				s.call(f, INT_TEXT);
				push_text(s, f, slash);
				concat(f);
				f.instruction(&I::LocalGet(denominator));
				s.call(f, INT_TEXT);
				concat(f);
				Self::emit_list(f, &[I::Return, I::End]);
				// scale = the smallest power of ten that makes x an integer, rest = |x| * scale
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Const(1), I::LocalSet(scale),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(numerator), I::LocalGet(scale),
				]);
				s.call(f, "int_mul");
				f.instruction(&I::LocalGet(denominator));
				s.call(f, "int_rem");
				Self::emit_list(f, &[I::I64Eqz, I::BrIf(1), I::LocalGet(scale), I::I64Const(DECIMAL_BASE)]);
				s.call(f, "int_mul");
				Self::emit_list(
					f,
					&[I::LocalSet(scale), I::Br(0), I::End, I::End, I::LocalGet(numerator), I::LocalGet(scale)],
				);
				s.call(f, "int_mul");
				f.instruction(&I::LocalGet(denominator));
				s.call(f, "int_quot");
				f.instruction(&I::LocalSet(rest));
				s.is_negative(f, rest);
				f.instruction(&I::If(BlockType::Result(node_ref)));
				s.negate_local(f, rest);
				push_text(s, f, minus);
				f.instruction(&I::Else);
				push_text(s, f, empty);
				Self::emit_list(f, &[I::End, I::LocalGet(rest), I::LocalGet(scale)]);
				s.call(f, "int_quot");
				s.call(f, INT_TEXT);
				concat(f);
				push_text(s, f, dot);
				concat(f);
				// the fraction digits with their leading zeros: the text of scale + fraction without its leading 1
				Self::emit_list(f, &[I::LocalGet(rest), I::LocalGet(scale)]);
				s.call(f, "int_rem");
				f.instruction(&I::LocalGet(scale));
				s.call(f, "int_add");
				s.call(f, INT_TEXT);
				f.instruction(&I::LocalSet(fraction));
				s.emit_text_bytes(f, fraction);
				f.instruction(&I::I32Const(1));
				s.emit_text_bytes(f, fraction);
				Self::emit_list(f, &[I::ArrayLen, I::I32Const(1), I::I32Sub]);
				s.call(f, TEXT_PIECE);
				s.call(f, NEW_TEXT_OF);
				concat(f);
			},
		);

		if !self.should_emit_function(DECIMAL_TEXT) {
			return;
		}
		// decimal_text(x), written in warp: lib/prelude.warp decimal_text(numerator, denominator), called late with a
		// ratio's parts; an integer, ±∞ and NaN as exact_text writes them
		let late = self.late_call(DECIMAL_TEXT_WORD, vec![i64t, i64t], vec![node_ref]);
		self.runtime_function(DECIMAL_TEXT, vec![i64t], vec![node_ref], vec![i64t], |s, f| {
			let (value, denominator) = (0, 1);
			f.instruction(&I::LocalGet(value));
			s.call(f, "is_ratio");
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(value)]);
			s.call(f, "exact_denominator");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalTee(denominator), I::I64Eqz, I::I32Eqz, I::If(BlockType::Empty),
					I::LocalGet(value),
			]);
			s.call(f, "exact_numerator");
			f.instruction(&I::LocalGet(denominator));
			Self::emit_late_call(f, &late);
			Self::emit_list(f, &[I::Return, I::End, I::End, I::LocalGet(value)]);
			s.call(f, EXACT_TEXT);
		});
	}

	/// `while rest rem prime == 0 { rest = rest quot prime; each_time }` on the Int local `rest`
	fn divide_out(&self, f: &mut Function, rest: u32, prime: i64, each_time: &[I]) {
		Self::emit_list(
			f,
			&[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(rest), I::I64Const(prime)],
		);
		self.call(f, "int_rem");
		Self::emit_list(f, &[I::I64Eqz, I::I32Eqz, I::BrIf(1), I::LocalGet(rest), I::I64Const(prime)]);
		self.call(f, "int_quot");
		f.instruction(&I::LocalSet(rest));
		Self::emit_list(f, each_time);
		Self::emit_list(f, &[I::Br(0), I::End, I::End]);
	}

	/// Push numerator(x) quot denominator(x)
	fn part_quotient(&self, f: &mut Function, x: u32) {
		f.instruction(&I::LocalGet(x));
		self.call(f, "exact_numerator");
		f.instruction(&I::LocalGet(x));
		self.call(f, "exact_denominator");
		self.call(f, "int_quot");
	}
}

/// Exact fraction of the shortest decimal that round-trips `value`: 0.1 → (1, 10)
pub fn decimal_fraction(value: f64) -> (BigInt, BigInt) {
	crate::extensions::numbers::decimal_text_fraction(&format!("{:e}", value))
		.expect("LowerExp writes decimal digits and an exponent")
}
