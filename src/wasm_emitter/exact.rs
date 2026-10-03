//! Exact numbers by default: `1/3*3 == 1`, `0.1+0.2 == 0.3` (DESIGN.md "Exact numbers by default").
//!
//! An exact number is an Int (see big_int.rs) whose handle may also point at a `$Ratio` in the
//! same heap: numerator and denominator are Int payloads, the denominator is > 1 and coprime to
//! the numerator. A ratio is never integral and integers are never ratios, so integers keep the
//! i64 fast path and only the slow paths ask `is_ratio`. The `$Ratio` doubles as the payload of
//! an Int node, read back as `Number::Quotient`.
//!
//! Decimal literals are exact (`0.1` is 1/10, see `Number::is_exact_decimal`); f64 only comes from
//! float functions (`sqrt`, FFI), irrational constants (`π`) or `as float`, and mixing an f64 into an
//! expression makes the result an f64.
//! Division by zero yields the extended rationals' ±∞ = ±1/0 and NaN = 0/0 (never a quiet f64 NaN):
//! ∞ + 1 = ∞, 1/∞ = 0, ∞ - ∞ = NaN, NaN equals only NaN. Truncating ∞ or NaN to an integer traps.

use super::WasmGcEmitter;
use num_bigint::BigInt;
use num_traits::{One, Pow};
use wasm_encoder::*;
use Instruction as I;
use crate::wasm_emitter::layout::BYTE;

/// Largest shift count: `1 << n` allocates n bits, a bigger count is a runtime error instead of an out-of-memory
pub const MAX_SHIFT_COUNT: i64 = 1 << 16;
/// exact_text(x): the text of any exact number, as the host prints it (`2.5`, `1/3`, `-∞`, big integers in full)
pub const EXACT_TEXT: &str = "exact_text";
const INT_TEXT: &str = "int_text";
const DECIMAL_BASE: i64 = 10;
/// A ratio whose denominator has no other prime factors is a terminating decimal
const DECIMAL_PRIMES: [i64; 2] = [2, 5];

impl WasmGcEmitter {
	/// Decimal literal as the exact value of its shortest round-trip digits: `0.1` → 1/10
	pub(crate) fn emit_decimal_literal(&mut self, func: &mut Function, value: f64) {
		let (numerator, denominator) = decimal_fraction(value);
		self.emit_exact_literal(func, &numerator, &denominator);
	}

	pub(crate) fn emit_exact_literal(&mut self, func: &mut Function, numerator: &BigInt, denominator: &BigInt) {
		self.emit_int_literal(func, numerator);
		if denominator.is_one() {
			return;
		}
		self.emit_int_literal(func, denominator);
		self.emit_call(func, "exact_div");
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

	/// `name(a, b)`: ratio_op when an operand is a ratio, else the integer slow path
	fn exact_dispatch(&mut self, name: &'static str, result: ValType, ratio_op: impl FnOnce(&Self, &mut Function), integer_op: &str) {
		let i64t = ValType::I64;
		self.runtime_function(name, vec![i64t, i64t], vec![result], vec![], |s, f| {
			s.any_ratio(f, 0, 1);
			f.instruction(&I::If(BlockType::Result(result)));
			ratio_op(s, f);
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, integer_op);
			f.instruction(&I::End);
		});
	}

	/// `name(x)`: ratio_op on a ratio, else the integer slow path
	fn exact_unary(&mut self, name: &'static str, result: ValType, ratio_op: impl FnOnce(&Self, &mut Function), integer_op: &str) {
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
		let (i64t, i32t, f64t) = (ValType::I64, ValType::I32, ValType::F64);
		let ratio = self.type_manager.ratio_type;
		let ratio_ref = self.ratio_ref();

		// ── integer helpers with inline fast paths ──
		for (name, machine, slow) in [("int_add", I::I64Add, "int_add_slow"), ("int_sub", I::I64Sub, "int_sub_slow")] {
			// locals: result
			self.runtime_function(name, vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1), machine, I::LocalSet(2)]);
				s.emit_fixnum_test(f, &[0, 1, 2]);
				Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(2), I::Else, I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, slow);
				f.instruction(&I::End);
			});
		}

		// int_quot / int_rem: truncated like i64.div_s / i64.rem_s, divisor nonzero
		for (name, machine, slow) in [("int_quot", I::I64DivS, "int_quot_slow"), ("int_rem", I::I64RemS, "int_rem_slow")] {
			self.runtime_function(name, vec![i64t, i64t], vec![i64t], vec![], |s, f| {
				s.emit_fixnum_test(f, &[0, 1]);
				Self::emit_list(f, &[I::If(BlockType::Result(i64t)), I::LocalGet(0), I::LocalGet(1), machine]);
				s.call(f, "int_from_i64"); // 2^62 / -1 leaves the fixnum range
				Self::emit_list(f, &[I::Else, I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, slow);
				f.instruction(&I::End);
			});
		}

		// int_gcd(a, b) >= 0 by Euclid; locals: remainder
		self.runtime_function("int_gcd", vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			for local in [0, 1] {
				s.is_negative(f, local);
				f.instruction(&I::If(BlockType::Empty));
				s.negate_local(f, local);
				f.instruction(&I::End);
			}
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(1), I::I64Eqz, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "int_rem");
			Self::emit_list(f, &[I::LocalSet(2), I::LocalGet(1), I::LocalSet(0), I::LocalGet(2), I::LocalSet(1), I::Br(0), I::End, I::End]);
			f.instruction(&I::LocalGet(0));
		});

		// ── ratios ──
		self.runtime_function("is_ratio", vec![i64t], vec![i32t], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0]);
			Self::emit_list(f, &[I::If(BlockType::Result(i32t)), I::I32Const(0), I::Else]);
			s.emit_heap_get(f, 0);
			Self::emit_list(f, &[I::RefTestNonNull(ratio_ref), I::End]);
		});

		for (name, field, integer_value) in [("exact_numerator", 0, None), ("exact_denominator", 1, Some(1))] {
			self.runtime_function(name, vec![i64t], vec![i64t], vec![], |s, f| {
				f.instruction(&I::LocalGet(0));
				s.call(f, "is_ratio");
				f.instruction(&I::If(BlockType::Result(i64t)));
				s.emit_heap_get(f, 0);
				Self::emit_list(f, &[I::RefCastNonNull(ratio_ref), I::StructGet { struct_type_index: ratio, field_index: field }]);
				s.call(f, "int_from_payload");
				f.instruction(&I::Else);
				f.instruction(&match integer_value {
					Some(one) => I::I64Const(one),
					None => I::LocalGet(0),
				});
				f.instruction(&I::End);
			});
		}

		// ratio_new(n, d): normalized n/d for integers, d nonzero; locals: gcd
		self.runtime_function("ratio_new", vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			s.is_negative(f, 1);
			f.instruction(&I::If(BlockType::Empty));
			s.negate_local(f, 0);
			s.negate_local(f, 1);
			f.instruction(&I::End);
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "int_gcd");
			// gcd 0 only for 0/0 (NaN)
			Self::emit_list(f, &[I::LocalTee(2), I::I64Const(1), I::I64GtS, I::If(BlockType::Empty)]);
			for local in [0, 1] {
				Self::emit_list(f, &[I::LocalGet(local), I::LocalGet(2)]);
				s.call(f, "int_quot");
				f.instruction(&I::LocalSet(local));
			}
			f.instruction(&I::End);
			Self::emit_list(f, &[I::LocalGet(1), I::I64Const(1), I::I64Eq, I::If(BlockType::Result(i64t)), I::LocalGet(0), I::Else]);
			s.emit_int_payload(f, 0);
			s.emit_int_payload(f, 1);
			f.instruction(&I::StructNew(ratio));
			s.call(f, "int_store");
			f.instruction(&I::End);
		});

		// exact_div(a, b): a/b, an integer whenever b divides a; locals: quotient, remainder
		let (quotient, remainder) = (2, 3);
		self.runtime_function("exact_div", vec![i64t, i64t], vec![i64t], vec![i64t, i64t], |s, f| {
			let ratio_of_both = |f: &mut Function| {
				s.cross_product(f, 0, 1);
				s.cross_product(f, 1, 0);
				s.call(f, "ratio_new");
			};
			s.emit_fixnum_test(f, &[0, 1]);
			Self::emit_list(f, &[
				I::If(BlockType::Result(i64t)),
				I::LocalGet(1), I::I64Eqz, I::If(BlockType::Result(i32t)), I::I32Const(0), I::Else,
				I::LocalGet(0), I::LocalGet(1), I::I64RemS, I::I64Eqz, I::End,
				I::If(BlockType::Result(i64t)), I::LocalGet(0), I::LocalGet(1), I::I64DivS,
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
			Self::emit_list(f, &[
				I::LocalSet(remainder), I::LocalSet(quotient),
				I::LocalGet(remainder), I::I64Eqz, I::If(BlockType::Result(i64t)), I::LocalGet(quotient), I::Else,
			]);
			ratio_of_both(f);
			Self::emit_list(f, &[I::End, I::End, I::End]);
		});

		// ── dispatch from the Int slow paths ──
		for (name, combine, integer_op) in [("exact_add", "int_add", "int_add_slow"), ("exact_sub", "int_sub", "int_sub_slow")] {
			self.exact_dispatch(name, i64t, |s, f| {
				s.cross_product(f, 0, 1);
				s.cross_product(f, 1, 0);
				s.call(f, combine);
				s.part_product(f, 0, "exact_denominator", 1, "exact_denominator");
				s.call(f, "ratio_new");
			}, integer_op);
		}
		self.exact_dispatch("exact_mul", i64t, |s, f| {
			s.part_product(f, 0, "exact_numerator", 1, "exact_numerator");
			s.part_product(f, 0, "exact_denominator", 1, "exact_denominator");
			s.call(f, "ratio_new");
		}, "int_mul_slow");
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
		self.exact_dispatch("exact_cmp", i32t, |s, f| {
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
		}, "int_cmp");

		self.exact_unary("exact_neg", i64t, |s, f| {
			Self::emit_list(f, &[I::I64Const(0), I::LocalGet(0)]);
			s.call(f, "exact_numerator");
			s.call(f, "int_sub");
			f.instruction(&I::LocalGet(0));
			s.call(f, "exact_denominator");
			s.call(f, "ratio_new");
		}, "int_neg_slow");
		self.exact_unary("exact_abs", i64t, |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::I64Const(0)]);
			s.call(f, "exact_cmp");
			Self::emit_list(f, &[I::I32Const(0), I::I32LtS, I::If(BlockType::Result(i64t)), I::LocalGet(0)]);
			s.call(f, "exact_neg");
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::End]);
		}, "int_abs_slow");
		self.exact_unary("exact_to_f64", f64t, |s, f| {
			f.instruction(&I::LocalGet(0));
			s.call(f, "exact_numerator");
			s.call(f, "int_to_f64");
			f.instruction(&I::LocalGet(0));
			s.call(f, "exact_denominator");
			s.call(f, "int_to_f64");
			f.instruction(&I::F64Div);
		}, "int_to_f64");

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
		self.exact_dispatch("exact_rem", i64t, |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "exact_div");
			s.call(f, "exact_trunc");
			s.call(f, "exact_mul");
			s.call(f, "exact_sub");
		}, "int_rem_slow");

		// exact_mod(a, b): Euclidean remainder, 0 ≤ r < |b|: the truncated remainder plus |b| when negative; locals: r
		self.runtime_function("exact_mod", vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, "exact_rem");
			Self::emit_list(f, &[I::LocalTee(2), I::I64Const(0)]);
			s.call(f, "exact_cmp");
			Self::emit_list(f, &[I::I32Const(0), I::I32LtS, I::If(BlockType::Result(i64t)), I::LocalGet(2), I::LocalGet(1)]);
			s.call(f, "exact_abs");
			s.call(f, "exact_add");
			Self::emit_list(f, &[I::Else, I::LocalGet(2), I::End]);
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
			f.instruction(&I::If(BlockType::Empty));
			Self::emit_list(f, &[I::I64Const(1), I::LocalGet(0)]);
			s.call(f, "exact_div");
			f.instruction(&I::LocalSet(0));
			s.negate_local(f, 1);
			f.instruction(&I::End);
			f.instruction(&I::LocalGet(0));
			s.call(f, "is_ratio");
			f.instruction(&I::If(BlockType::Result(i64t)));
			for part in ["exact_numerator", "exact_denominator"] {
				f.instruction(&I::LocalGet(0));
				s.call(f, part);
				f.instruction(&I::LocalGet(1));
				s.call(f, "int_pow");
			}
			s.call(f, "ratio_new");
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
		self.emit_text_heap_global();
		let (i64t, i32t, node_ref) = (ValType::I64, ValType::I32, ValType::Ref(self.node_ref(false)));
		let constant = |s: &mut Self, text: &str| s.allocate_string(text);
		let [minus, dot, slash, empty, infinity, negative_infinity, not_a_number] =
			["-", ".", "/", "", "∞", "-∞", "NaN"].map(|text| constant(self, text));
		let push_text = |s: &Self, f: &mut Function, (address, length): (u32, u32)| {
			Self::emit_list(f, &[I::I32Const(address as i32), I::I32Const(length as i32)]);
			s.call(f, "new_text");
		};

		// int_text(n): sign, then the digits written from the last one back; locals: negative, digits, rest, address, length
		self.runtime_function(INT_TEXT, vec![i64t], vec![node_ref], vec![i32t, i32t, i64t, i32t, i32t], |s, f| {
			let (value, negative, digits, rest, address, length) = (0, 1, 2, 3, 4, 5);
			s.is_negative(f, value);
			Self::emit_list(f, &[I::LocalTee(negative), I::If(BlockType::Empty)]);
			s.negate_local(f, value);
			Self::emit_list(f, &[I::End, I::I32Const(1), I::LocalSet(digits), I::LocalGet(value), I::LocalSet(rest)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(rest), I::I64Const(DECIMAL_BASE)]);
			s.call(f, "int_cmp");
			Self::emit_list(f, &[I::I32Const(0), I::I32LtS, I::BrIf(1), I::LocalGet(rest), I::I64Const(DECIMAL_BASE)]);
			s.call(f, "int_quot");
			Self::emit_list(f, &[I::LocalSet(rest), I::LocalGet(digits), I::I32Const(1), I::I32Add, I::LocalSet(digits), I::Br(0), I::End, I::End]);
			Self::emit_list(f, &[I::LocalGet(digits), I::LocalGet(negative), I::I32Add, I::LocalTee(length), I::LocalSet(digits)]);
			s.emit_text_allocation(f, length, address);
			Self::emit_list(f, &[I::LocalGet(negative), I::If(BlockType::Empty), I::LocalGet(address), I::I32Const(b'-' as i32), I::I32Store8(BYTE), I::End]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(digits), I::LocalGet(negative), I::I32LeU, I::BrIf(1)]);
			Self::emit_list(f, &[I::LocalGet(digits), I::I32Const(1), I::I32Sub, I::LocalTee(digits), I::LocalGet(address), I::I32Add]);
			Self::emit_list(f, &[I::LocalGet(value), I::I64Const(DECIMAL_BASE)]);
			s.call(f, "int_rem");
			Self::emit_list(f, &[I::I32WrapI64, I::I32Const(b'0' as i32), I::I32Add, I::I32Store8(BYTE), I::LocalGet(value), I::I64Const(DECIMAL_BASE)]);
			s.call(f, "int_quot");
			Self::emit_list(f, &[I::LocalSet(value), I::Br(0), I::End, I::End, I::LocalGet(address), I::LocalGet(length)]);
			s.call(f, "new_text");
		});

		// exact_text(x); locals: numerator, denominator, rest, scale, fraction digits
		let nullable = ValType::Ref(self.node_ref(true));
		self.runtime_function(EXACT_TEXT, vec![i64t], vec![node_ref], vec![i64t, i64t, i64t, i64t, nullable], |s, f| {
			let (value, numerator, denominator, rest, scale, fraction) = (0, 1, 2, 3, 4, 5);
			let concat = |f: &mut Function| s.call(f, crate::wasm_emitter::text_builtins::TEXT_CONCAT);
			f.instruction(&I::LocalGet(value));
			s.call(f, "is_ratio");
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(value)]);
			s.call(f, INT_TEXT);
			Self::emit_list(f, &[I::Return, I::End, I::LocalGet(value)]);
			s.call(f, "exact_numerator");
			Self::emit_list(f, &[I::LocalSet(numerator), I::LocalGet(value)]);
			s.call(f, "exact_denominator");
			// ±∞ = ±1/0 and NaN = 0/0
			Self::emit_list(f, &[I::LocalTee(denominator), I::I64Eqz, I::If(BlockType::Empty), I::LocalGet(numerator), I::I64Eqz, I::If(BlockType::Result(node_ref))]);
			push_text(s, f, not_a_number);
			Self::emit_list(f, &[I::Else, I::LocalGet(numerator), I::I64Const(0), I::I64GtS, I::If(BlockType::Result(node_ref))]);
			push_text(s, f, infinity);
			f.instruction(&I::Else);
			push_text(s, f, negative_infinity);
			Self::emit_list(f, &[I::End, I::End, I::Return, I::End]);
			// a denominator with a prime other than 2 and 5 never terminates: numerator/denominator
			Self::emit_list(f, &[I::LocalGet(denominator), I::LocalSet(rest)]);
			for prime in DECIMAL_PRIMES {
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(rest), I::I64Const(prime)]);
				s.call(f, "int_rem");
				Self::emit_list(f, &[I::I64Eqz, I::I32Eqz, I::BrIf(1), I::LocalGet(rest), I::I64Const(prime)]);
				s.call(f, "int_quot");
				Self::emit_list(f, &[I::LocalSet(rest), I::Br(0), I::End, I::End]);
			}
			Self::emit_list(f, &[I::LocalGet(rest), I::I64Const(1), I::I64Ne, I::If(BlockType::Empty), I::LocalGet(numerator)]);
			s.call(f, INT_TEXT);
			push_text(s, f, slash);
			concat(f);
			f.instruction(&I::LocalGet(denominator));
			s.call(f, INT_TEXT);
			concat(f);
			Self::emit_list(f, &[I::Return, I::End]);
			// scale = the smallest power of ten that makes x an integer, rest = |x| * scale
			Self::emit_list(f, &[I::I64Const(1), I::LocalSet(scale)]);
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(numerator), I::LocalGet(scale)]);
			s.call(f, "int_mul");
			f.instruction(&I::LocalGet(denominator));
			s.call(f, "int_rem");
			Self::emit_list(f, &[I::I64Eqz, I::BrIf(1), I::LocalGet(scale), I::I64Const(DECIMAL_BASE)]);
			s.call(f, "int_mul");
			Self::emit_list(f, &[I::LocalSet(scale), I::Br(0), I::End, I::End, I::LocalGet(numerator), I::LocalGet(scale)]);
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
			s.emit_text_field(f, fraction, 0);
			Self::emit_list(f, &[I::I32Const(1), I::I32Add]);
			s.emit_text_field(f, fraction, 1);
			Self::emit_list(f, &[I::I32Const(1), I::I32Sub]);
			s.call(f, "new_text");
			concat(f);
		});
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
	let scientific = format!("{:e}", value); // "-1.25e-3"
	let (mantissa, exponent) = scientific.split_once('e').expect("LowerExp has an exponent");
	let fraction_digits = mantissa.split_once('.').map_or(0, |(_, fraction)| fraction.len()) as i64;
	let digits: BigInt = mantissa.replace('.', "").parse().expect("decimal digits");
	let scale = exponent.parse::<i64>().expect("decimal exponent") - fraction_digits;
	let power: BigInt = BigInt::from(10).pow(scale.unsigned_abs() as u32);
	if scale >= 0 {
		(digits * power, BigInt::one())
	} else {
		(digits, power)
	}
}
