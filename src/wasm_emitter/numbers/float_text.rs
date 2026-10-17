//! float_text(x): the text of a run-time float, its shortest digits that read back (user decision float-text: `0.1+0.2`
//! as floats is `0.30000000000000004`, as JavaScript and Python write it), positional from 1e-5 up to 1e15 and as
//! `1.5e20` / `1e-7` outside; ∞, -∞ and NaN as the exact numbers print them.
//! The digits are found in wasm, no host needed (float_digits): x and the halfways to its neighbours, scaled by a power
//! of ten to 17 digits as double-doubles (float_scaled, Dekker's exact product), bound the whole numbers that read back
//! as x; the one with the most trailing zeros, nearest x, is the shortest. probes/floats/float_scaled_sim.py checks the
//! same steps against Python's repr.
//! list_join calls it, so print, `str(x)`, `x as text` and `"a" + x` all show floats the same way.

use crate::extensions::numbers::{LARGEST_POSITIONAL, SMALLEST_POSITIONAL};
use crate::wasm_emitter::layout::BYTE;
use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;

pub const FLOAT_TEXT: &str = "float_text";
/// float_scaled(x, gap, shift): (x + gap/2) · 10^shift as a double-double (hi, lo)
pub const FLOAT_SCALED: &str = "float_scaled";
/// 2^27 + 1 splits a float into two halves of 26 bits, whose products are exact (Veltkamp)
const SPLITTER: f64 = 134217729.0;
/// float_digits(x): the shortest digits of a positive float and their shift: digits · 10^-shift reads back as x
pub const FLOAT_DIGITS: &str = "float_digits";
/// float_as_exact(x): `x as rational` of a run-time float
pub const FLOAT_AS_EXACT: &str = "float_as_exact";
/// The runtime error of float_as_exact for ∞ and NaN
pub const NOT_FINITE: &str = "not_a_finite_number";
/// Powers of ten up to 10^22 are exact floats: float_scaled scales by one a step
const EXACT_POWER: i32 = 22;
/// A mantissa in [1, 10) times 10^MANTISSA_SHIFT is a whole number with 17 digits, from FEWEST_17 up to MOST_17
const MANTISSA_SHIFT: i32 = 16;
const FEWEST_17: f64 = 1e16;
const MOST_17: f64 = 1e17;
/// float_scaled moves a float beyond ±2^RESCALE_BEYOND by 2^∓RESCALE_BY first: no product overflows, no half underflows
const RESCALE_BEYOND: i32 = 900;
const RESCALE_BY: i32 = 128;
/// Sign, "0.", four zeros and the digits fit, so does the exponent form; the digits are formatted behind them
const TEXT_BYTES: i32 = 32;
const SCRATCH_BYTES: i32 = 20;
const COPY_BYTES: I<'static> = I::MemoryCopy { src_mem: 0, dst_mem: 0 };

impl WasmGcEmitter {
	pub(in crate::wasm_emitter) fn emit_float_text(&mut self) {
		if !self.should_emit_function(FLOAT_TEXT) {
			return;
		}
		self.emit_float_shortest_digits();
		self.emit_text_heap_global();
		let node_ref = ValType::Ref(self.node_ref(false));
		let [zero, infinity, negative_infinity, not_a_number] =
			["0", "∞", "-∞", "NaN"].map(|text| self.memory_string(text));
		let mut locals = vec![ValType::I32; 3];
		locals.extend([ValType::I64, ValType::I32, ValType::I32, ValType::I32]);
		self.runtime_function(FLOAT_TEXT, vec![ValType::F64], vec![node_ref], locals, |s, f| {
			let (x, address, position, exponent, digits, significant, count, shift) = (0, 1, 2, 3, 4, 5, 6, 7);
			let return_text = |f: &mut Function, (text_address, length): (u32, u32)| {
				Self::emit_list(f, &[I::I32Const(text_address as i32), I::I32Const(length as i32)]);
				s.call(f, "new_text");
				f.instruction(&I::Return);
			};
			let put = |f: &mut Function, byte: u8| {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(position), I::I32Const(byte as i32), I::I32Store8(BYTE),
					I::LocalGet(position), I::I32Const(1), I::I32Add, I::LocalSet(position),
				]);
			};
			// the digits are written to the scratch bytes behind the text, then copied into place
			let scratch = |f: &mut Function, offset: &[I<'static>]| {
				Self::emit_list(f, &[I::LocalGet(address), I::I32Const(TEXT_BYTES), I::I32Add]);
				Self::emit_list(f, offset);
				f.instruction(&I::I32Add);
			};
			let copy = |f: &mut Function, offset: &[I<'static>], length: &[I<'static>]| {
				f.instruction(&I::LocalGet(position));
				scratch(f, offset);
				Self::emit_list(f, length);
				Self::emit_list(f, &[COPY_BYTES, I::LocalGet(position)]);
				Self::emit_list(f, length);
				Self::emit_list(f, &[I::I32Add, I::LocalSet(position)]);
			};
			let zeros = |f: &mut Function, how_many: &[I<'static>]| {
				Self::emit_list(f, how_many);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalSet(count),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(count), I::I32Const(0), I::I32LeS, I::BrIf(1),
				]);
				put(f, b'0');
				Self::emit_list(
					f,
					&[I::LocalGet(count), I::I32Const(1), I::I32Sub, I::LocalSet(count), I::Br(0), I::End, I::End],
				);
			};

			Self::emit_list(f, &[I::LocalGet(x), I::LocalGet(x), I::F64Ne, I::If(BlockType::Empty)]);
			return_text(f, not_a_number);
			Self::emit_list(
				f,
				&[I::End, I::LocalGet(x), I::F64Const(f64::INFINITY.into()), I::F64Eq, I::If(BlockType::Empty)],
			);
			return_text(f, infinity);
			Self::emit_list(
				f,
				&[I::End, I::LocalGet(x), I::F64Const(f64::NEG_INFINITY.into()), I::F64Eq, I::If(BlockType::Empty)],
			);
			return_text(f, negative_infinity);
			Self::emit_list(f, &[I::End, I::LocalGet(x), I::F64Const(0.0.into()), I::F64Eq, I::If(BlockType::Empty)]);
			return_text(f, zero);
			f.instruction(&I::End);

			Self::emit_list(f, &[I::I32Const(TEXT_BYTES + SCRATCH_BYTES), I::LocalSet(count)]);
			s.emit_text_allocation(f, count, address);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(address), I::LocalSet(position),
				I::LocalGet(x), I::F64Const(0.0.into()), I::F64Lt, I::If(BlockType::Empty),
			]);
			put(f, b'-');
			Self::emit_list(f, &[I::LocalGet(x), I::F64Neg, I::LocalSet(x), I::End]);

			f.instruction(&I::LocalGet(x));
			s.call(f, FLOAT_DIGITS);
			Self::emit_list(f, &[I::LocalSet(shift), I::LocalSet(digits)]);
			scratch(f, &[I::I32Const(0)]);
			f.instruction(&I::LocalGet(digits));
			s.call(f, "int_to_decimal");
			// how many digits it wrote, and the exponent of the first
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalTee(significant), I::I32Const(1), I::I32Sub, I::LocalGet(shift), I::I32Sub,
				I::LocalSet(exponent),
			]);

			let all = [I::LocalGet(significant)];
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(exponent), I::I32Const(SMALLEST_POSITIONAL), I::I32GeS, I::LocalGet(exponent),
				I::I32Const(LARGEST_POSITIONAL), I::I32LtS, I::I32And, I::If(BlockType::Empty),
			]);
			{
				// 1500: the digits, then zeros
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(exponent), I::LocalGet(significant), I::I32Const(1), I::I32Sub, I::I32GeS,
					I::If(BlockType::Empty),
				]);
				copy(f, &[I::I32Const(0)], &all);
				zeros(f, &[I::LocalGet(exponent), I::LocalGet(significant), I::I32Sub, I::I32Const(1), I::I32Add]);
				// 1.5: the point after exponent+1 digits
				Self::emit_list(
					f,
					&[I::Else, I::LocalGet(exponent), I::I32Const(0), I::I32GeS, I::If(BlockType::Empty)],
				);
				let whole = [I::LocalGet(exponent), I::I32Const(1), I::I32Add];
				copy(f, &[I::I32Const(0)], &whole);
				put(f, b'.');
				copy(
					f,
					&whole,
					&[I::LocalGet(significant), I::LocalGet(exponent), I::I32Sub, I::I32Const(1), I::I32Sub],
				);
				// 0.0015: zero, point, zeros, the digits
				f.instruction(&I::Else);
				put(f, b'0');
				put(f, b'.');
				zeros(f, &[I::I32Const(-1), I::LocalGet(exponent), I::I32Sub]);
				copy(f, &[I::I32Const(0)], &all);
				Self::emit_list(f, &[I::End, I::End]);
			}
			// 1.5e20: one digit, the point and the rest, then the exponent
			f.instruction(&I::Else);
			copy(f, &[I::I32Const(0)], &[I::I32Const(1)]);
			Self::emit_list(f, &[I::LocalGet(significant), I::I32Const(1), I::I32GtS, I::If(BlockType::Empty)]);
			put(f, b'.');
			copy(f, &[I::I32Const(1)], &[I::LocalGet(significant), I::I32Const(1), I::I32Sub]);
			f.instruction(&I::End);
			put(f, b'e');
			Self::emit_list(
				f,
				&[I::LocalGet(position), I::LocalGet(position), I::LocalGet(exponent), I::I64ExtendI32S],
			);
			s.call(f, "int_to_decimal");
			Self::emit_list(f, &[I::I32Add, I::LocalSet(position), I::End]);

			s.emit_new_text_to(f, address, position);
		});
	}

	/// float_digits and the float_scaled it calls, once for float_text and float_as_exact
	fn emit_float_shortest_digits(&mut self) {
		if !self.ctx.func_registry.contains(FLOAT_DIGITS) {
			self.emit_float_scaled();
			self.emit_float_digits();
		}
	}

	/// float_as_exact(x): the exact value of a float's shortest digits, `x as rational` (decision float-as-rational):
	/// 1.4142135623730951 is 14142135623730951/10^16, reduced by exact_div; ∞ and NaN have no fraction
	pub(in crate::wasm_emitter) fn emit_float_as_exact(&mut self) {
		if !self.should_emit_function(FLOAT_AS_EXACT) {
			return;
		}
		self.emit_float_shortest_digits();
		let i64t = ValType::I64;
		self.runtime_function(FLOAT_AS_EXACT, vec![ValType::F64], vec![i64t], vec![i64t, ValType::I32], |s, f| {
			let (x, digits, shift) = (0, 1, 2);
			// x - x is 0 for every finite x, NaN for ∞ and NaN
			Self::emit_list(f, &[I::LocalGet(x), I::LocalGet(x), I::F64Sub, I::F64Const(0.0.into()), I::F64Ne]);
			s.emit_fail_if(f, NOT_FINITE);
			Self::emit_list(
				f,
				&[
					I::LocalGet(x),
					I::F64Const(0.0.into()),
					I::F64Eq,
					I::If(BlockType::Empty),
					I::I64Const(0),
					I::Return,
					I::End,
					I::LocalGet(x),
					I::F64Abs,
				],
			);
			s.call(f, FLOAT_DIGITS);
			Self::emit_list(
				f,
				&[
					I::LocalSet(shift),
					I::LocalSet(digits),
					I::LocalGet(x),
					I::F64Const(0.0.into()),
					I::F64Lt,
					I::If(BlockType::Empty),
					I::I64Const(0),
					I::LocalGet(digits),
					I::I64Sub,
					I::LocalSet(digits),
					I::End,
					I::LocalGet(digits),
				],
			);
			s.call(f, "int_from_i64");
			// digits · 10^-shift: divided by 10^shift, or multiplied by 10^-shift
			let power_of_ten = |f: &mut Function, exponent: &[I<'static>]| {
				Self::emit_list(f, &[I::LocalGet(digits), I::I64Const(10)]);
				Self::emit_list(f, exponent);
				s.call(f, "int_pow");
			};
			Self::emit_list(
				f,
				&[I::LocalSet(digits), I::LocalGet(shift), I::I32Const(0), I::I32GeS, I::If(BlockType::Result(i64t))],
			);
			power_of_ten(f, &[I::LocalGet(shift), I::I64ExtendI32S]);
			s.call(f, "exact_div");
			f.instruction(&I::Else);
			power_of_ten(f, &[I::I32Const(0), I::LocalGet(shift), I::I32Sub, I::I64ExtendI32S]);
			s.call(f, "int_mul");
			f.instruction(&I::End);
		});
	}

	/// float_digits(x), x positive: 17 digits of x (the shift one more or less until it has them) and the whole numbers
	/// between the halfways to its neighbours, which read back as x (the halfways too when x's last bit is even, as a
	/// reader rounds half to even); the multiple of the largest power of ten among them, nearest x (half to even)
	fn emit_float_digits(&mut self) {
		let (f64t, i64t) = (ValType::F64, ValType::I64);
		let locals = vec![ValType::I32, ValType::I32, f64t, f64t, i64t, f64t, i64t, i64t, i64t, f64t, f64t, f64t, i64t];
		self.runtime_function(FLOAT_DIGITS, vec![f64t], vec![i64t, ValType::I32], locals, |s, f| {
			let (x, exponent, shift, hi, lo, whole, fraction, low, high, power, mantissa, below, above, multiple) =
				(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
			let scaled = |f: &mut Function, gap: &[I<'static>]| {
				Self::emit_list(f, &[I::LocalGet(x)]);
				Self::emit_list(f, gap);
				Self::emit_list(f, &[I::LocalGet(shift)]);
				s.call(f, FLOAT_SCALED);
				Self::emit_list(f, &[I::LocalSet(lo), I::LocalSet(hi)]);
			};
			// x = mantissa · 10^exponent, the exponent perhaps one off
			Self::emit_list(f, &[I::LocalGet(x), I::LocalSet(mantissa)]);
			let step = |op: I<'static>, exponent_step: i32| {
				[
					I::LocalGet(mantissa),
					I::F64Const(10.0.into()),
					op,
					I::LocalSet(mantissa),
					I::LocalGet(exponent),
					I::I32Const(exponent_step),
					I::I32Add,
					I::LocalSet(exponent),
				]
			};
			Self::emit_while(f, &[I::LocalGet(mantissa), I::F64Const(10.0.into()), I::F64Ge], &step(I::F64Div, 1));
			Self::emit_while(f, &[I::LocalGet(mantissa), I::F64Const(1.0.into()), I::F64Lt], &step(I::F64Mul, -1));
			// x · 10^shift has 17 digits
			let at_least = |bound: f64| {
				[
					I::LocalGet(hi),
					I::F64Const(bound.into()),
					I::F64Gt,
					I::LocalGet(hi),
					I::F64Const(bound.into()),
					I::F64Eq,
					I::LocalGet(lo),
					I::F64Const(0.0.into()),
					I::F64Ge,
					I::I32And,
					I::I32Or,
				]
			};
			let shift_by =
				|step: i32| [I::LocalGet(shift), I::I32Const(step), I::I32Add, I::LocalSet(shift), I::Br(1), I::End];
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Const(MANTISSA_SHIFT), I::LocalGet(exponent), I::I32Sub, I::LocalSet(shift),
				I::Loop(BlockType::Empty),
			]);
			scaled(f, &[I::F64Const(0.0.into())]);
			Self::emit_list(f, &at_least(MOST_17));
			f.instruction(&I::If(BlockType::Empty));
			Self::emit_list(f, &shift_by(-1));
			Self::emit_list(f, &at_least(FEWEST_17));
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			Self::emit_list(f, &shift_by(1));
			f.instruction(&I::End);
			// hi is whole beyond 2^53: x · 10^shift is whole + fraction
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(hi), I::I64TruncF64S, I::LocalGet(lo), I::F64Floor, I::I64TruncF64S, I::I64Add,
				I::LocalSet(whole),
				I::LocalGet(lo), I::LocalGet(lo), I::F64Floor, I::F64Sub, I::LocalSet(fraction),
			]);
			// the gaps to the neighbours; the largest float's upper one is its lower
			let neighbour =
				|step: i64| [I::LocalGet(x), I::I64ReinterpretF64, I::I64Const(step), I::I64Add, I::F64ReinterpretI64];
			Self::emit_list(f, &[I::LocalGet(x)]);
			Self::emit_list(f, &neighbour(-1));
			Self::emit_list(f, &[I::F64Sub, I::LocalTee(below)]);
			Self::emit_list(f, &neighbour(1));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(x), I::F64Sub, I::LocalTee(above), I::LocalGet(above), I::F64Const(f64::INFINITY.into()),
				I::F64Eq, I::Select, I::LocalSet(above),
			]);
			// an odd last bit excludes a halfway that is whole
			let odd_and_whole = |f: &mut Function, rounded: I<'static>| {
				Self::emit_list(
					f,
					&[
						I::LocalGet(x),
						I::I64ReinterpretF64,
						I::I32WrapI64,
						I::I32Const(1),
						I::I32And,
						I::LocalGet(lo),
						I::LocalGet(lo),
						rounded,
						I::F64Eq,
						I::I32And,
						I::I64ExtendI32U,
					],
				);
			};
			let bound = |f: &mut Function, rounded: I<'static>, local: u32, exclude: I<'static>| {
				Self::emit_list(
					f,
					&[I::LocalGet(hi), I::I64TruncF64S, I::LocalGet(lo), rounded.clone(), I::I64TruncF64S, I::I64Add],
				);
				odd_and_whole(f, rounded);
				Self::emit_list(f, &[exclude, I::LocalSet(local)]);
			};
			scaled(f, &[I::LocalGet(below), I::F64Neg]);
			bound(f, I::F64Ceil, low, I::I64Add);
			scaled(f, &[I::LocalGet(above)]);
			bound(f, I::F64Floor, high, I::I64Sub);
			// the largest power of ten with a multiple from low to high
			#[rustfmt::skip]
			let lowest_multiple = [
				I::LocalGet(low), I::LocalGet(power), I::I64Add, I::I64Const(1), I::I64Sub, I::LocalGet(power),
				I::I64DivS, I::LocalGet(power), I::I64Mul,
			];
			Self::emit_list(f, &[I::I64Const(10), I::LocalSet(power)]);
			Self::emit_while(
				f,
				&[&lowest_multiple[..], &[I::LocalGet(high), I::I64LeS]].concat(),
				&[I::LocalGet(power), I::I64Const(10), I::I64Mul, I::LocalSet(power)],
			);
			Self::emit_list(f, &[I::LocalGet(power), I::I64Const(10), I::I64DivS, I::LocalSet(power)]);
			// its multiple nearest x · 10^shift: the one below or the next, by twice the distance minus the power
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(whole), I::LocalGet(whole), I::LocalGet(power), I::I64RemS, I::I64Sub,
				I::LocalSet(multiple),
				I::LocalGet(whole), I::LocalGet(multiple), I::I64Sub, I::I64Const(2), I::I64Mul, I::LocalGet(power),
				I::I64Sub, I::F64ConvertI64S, I::LocalGet(fraction), I::F64Const(2.0.into()), I::F64Mul, I::F64Add,
				I::LocalTee(fraction), I::F64Const(0.0.into()), I::F64Gt, I::LocalGet(fraction),
				I::F64Const(0.0.into()), I::F64Eq, I::LocalGet(multiple), I::LocalGet(power), I::I64DivS, I::I32WrapI64,
				I::I32Const(1), I::I32And, I::I32And, I::I32Or, I::I64ExtendI32U, I::LocalGet(power), I::I64Mul,
				I::LocalGet(multiple), I::I64Add, I::LocalSet(multiple),
			]);
			// kept from low to high
			let moved_into = |outside: I<'static>, bound: u32, step: I<'static>| {
				[
					I::LocalGet(multiple),
					I::LocalGet(bound),
					outside,
					I::If(BlockType::Empty),
					I::LocalGet(multiple),
					I::LocalGet(power),
					step,
					I::LocalSet(multiple),
					I::End,
				]
			};
			Self::emit_list(f, &moved_into(I::I64LtS, low, I::I64Add));
			Self::emit_list(f, &moved_into(I::I64GtS, high, I::I64Sub));
			// without trailing zeros
			let has_trailing_zero = [I::LocalGet(multiple), I::I64Const(10), I::I64RemS, I::I64Eqz];
			#[rustfmt::skip]
			let drop_trailing_zero = [
				I::LocalGet(multiple), I::I64Const(10), I::I64DivS, I::LocalSet(multiple),
				I::LocalGet(shift), I::I32Const(1), I::I32Sub, I::LocalSet(shift),
			];
			Self::emit_while(f, &has_trailing_zero, &drop_trailing_zero);
			Self::emit_list(f, &[I::LocalGet(multiple), I::LocalGet(shift)]);
		});
	}

	/// float_scaled(x, gap, shift): (x + gap/2) · 10^shift as hi + lo, by at most 10^22 a step (each power exact): a
	/// product's error is Dekker's exact product, a quotient's the remainder of its exact product. Exact in one step,
	/// within 2^-104 in more (beyond 1e±22). A float beyond ±2^900 is moved by 2^∓128 first and back after
	fn emit_float_scaled(&mut self) {
		let f64t = ValType::F64;
		let mut locals = vec![f64t; 11];
		locals.extend([ValType::I32, ValType::I32]);
		self.runtime_function(FLOAT_SCALED, vec![f64t, f64t, ValType::I32], vec![f64t, f64t], locals, |_, f| {
			let (x, gap, shift, hi, lo, power, carried, quotient, product, a_hi, a_lo, b_hi, b_lo, scale, step, count) =
				(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
			let rescaled = |beyond: I<'static>, exponent: i32| {
				let by = |local: u32| {
					[I::LocalGet(local), I::F64Const(2f64.powi(-exponent).into()), I::F64Mul, I::LocalSet(local)]
				};
				[
					&[
						I::LocalGet(x),
						I::F64Const(2f64.powi(exponent.signum() * RESCALE_BEYOND).into()),
						beyond,
						I::If(BlockType::Empty),
					][..],
					&by(x),
					&by(gap),
					&[I::F64Const(2f64.powi(exponent).into()), I::LocalSet(scale), I::End],
				]
				.concat()
			};
			Self::emit_list(f, &[I::F64Const(1.0.into()), I::LocalSet(scale)]);
			Self::emit_list(f, &rescaled(I::F64Ge, RESCALE_BY));
			Self::emit_list(f, &rescaled(I::F64Lt, -RESCALE_BY));
			Self::emit_list(f, &[I::LocalGet(gap), I::F64Const(0.5.into()), I::F64Mul, I::LocalSet(lo)]);
			let split = |f: &mut Function, value: u32, high: u32, low: u32| {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::F64Const(SPLITTER.into()), I::LocalGet(value), I::F64Mul, I::LocalTee(high), I::LocalGet(high),
					I::LocalGet(value), I::F64Sub, I::F64Sub, I::LocalSet(high),
					I::LocalGet(value), I::LocalGet(high), I::F64Sub, I::LocalSet(low),
				]);
			};
			// product = a · b rounded, its error left on the stack
			let exact_product = |f: &mut Function, a: u32, b: u32| {
				Self::emit_list(f, &[I::LocalGet(a), I::LocalGet(b), I::F64Mul, I::LocalSet(product)]);
				split(f, a, a_hi, a_lo);
				split(f, b, b_hi, b_lo);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(a_hi), I::LocalGet(b_hi), I::F64Mul, I::LocalGet(product), I::F64Sub, I::LocalGet(a_hi),
					I::LocalGet(b_lo), I::F64Mul, I::F64Add, I::LocalGet(a_lo), I::LocalGet(b_hi), I::F64Mul, I::F64Add,
					I::LocalGet(a_lo), I::LocalGet(b_lo), I::F64Mul, I::F64Add,
				]);
			};
			let clamp = |bound: i32, below: I<'static>| {
				[
					I::LocalGet(step),
					I::I32Const(bound),
					I::LocalGet(step),
					I::I32Const(bound),
					below,
					I::Select,
					I::LocalSet(step),
				]
			};
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(x), I::LocalSet(hi),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(shift), I::I32Eqz, I::BrIf(1),
						I::LocalGet(shift), I::LocalSet(step),
			]);
			Self::emit_list(f, &clamp(EXACT_POWER, I::I32LtS));
			Self::emit_list(f, &clamp(-EXACT_POWER, I::I32GtS));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Const(0), I::LocalGet(step), I::I32Sub, I::LocalGet(step), I::LocalGet(step), I::I32Const(0),
				I::I32LtS, I::Select, I::LocalSet(count),
				I::F64Const(1.0.into()), I::LocalSet(power),
				I::LocalGet(lo), I::LocalSet(carried),
			]);
			#[rustfmt::skip]
			Self::emit_while(f, &[I::LocalGet(count), I::I32Const(0), I::I32GtS], &[
				I::LocalGet(power), I::F64Const(10.0.into()), I::F64Mul, I::LocalSet(power),
				I::LocalGet(count), I::I32Const(1), I::I32Sub, I::LocalSet(count),
			]);
			Self::emit_list(f, &[I::LocalGet(step), I::I32Const(0), I::I32GtS, I::If(BlockType::Empty)]);
			exact_product(f, hi, power);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(carried), I::LocalGet(power), I::F64Mul, I::F64Add, I::LocalSet(lo),
				I::LocalGet(product), I::LocalSet(hi),
				I::Else,
					I::LocalGet(hi), I::LocalGet(power), I::F64Div, I::LocalSet(quotient),
			]);
			exact_product(f, quotient, power);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(b_lo),
				I::LocalGet(hi), I::LocalGet(product), I::F64Sub, I::LocalGet(b_lo), I::F64Sub, I::LocalGet(carried),
				I::F64Add, I::LocalGet(power), I::F64Div, I::LocalSet(lo),
				I::LocalGet(quotient), I::LocalSet(hi),
				I::End,
			]);
			// hi + lo renormalized, so hi is the nearest float
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(hi), I::LocalGet(lo), I::F64Add, I::LocalSet(product),
				I::LocalGet(lo), I::LocalGet(product), I::LocalGet(hi), I::F64Sub, I::F64Sub, I::LocalSet(lo),
				I::LocalGet(product), I::LocalSet(hi),
				I::LocalGet(shift), I::LocalGet(step), I::I32Sub, I::LocalSet(shift),
				I::Br(0),
				I::End,
				I::End,
				I::LocalGet(hi), I::LocalGet(scale), I::F64Mul, I::LocalGet(lo), I::LocalGet(scale), I::F64Mul,
			]);
		});
	}
}
