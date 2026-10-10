//! float_text(x): the text of a run-time float, with at most 15 significant digits (`0.1+0.2` is `0.3`, as `%.15g`),
//! positional from 1e-5 up to 1e15 and as `1.5e20` / `1e-7` outside; ∞, -∞ and NaN as the exact numbers print them.
//! list_join calls it, so print, `str(x)`, `x as text` and `"a" + x` all show floats the same way.

use crate::extensions::numbers::{LARGEST_POSITIONAL, MANTISSA_OVERFLOW, MANTISSA_SCALE, SIGNIFICANT_DIGITS, SMALLEST_POSITIONAL};
use crate::wasm_emitter::layout::BYTE;
use crate::wasm_emitter::WasmGcEmitter;
use wasm_encoder::*;
use Instruction as I;

pub const FLOAT_TEXT: &str = "float_text";
/// Sign, "0.", four zeros and the digits fit, so does the exponent form; the digits are formatted behind them
const TEXT_BYTES: i32 = 32;
const SCRATCH_BYTES: i32 = 16;
const COPY_BYTES: I<'static> = I::MemoryCopy { src_mem: 0, dst_mem: 0 };

impl WasmGcEmitter {
	pub(super) fn emit_float_text(&mut self) {
		if !self.should_emit_function(FLOAT_TEXT) {
			return;
		}
		self.emit_text_heap_global();
		let node_ref = ValType::Ref(self.node_ref(false));
		let [zero, infinity, negative_infinity, not_a_number] = ["0", "∞", "-∞", "NaN"].map(|text| self.allocate_string(text));
		let mut locals = vec![ValType::I32; 3];
		locals.extend([ValType::I64, ValType::I32, ValType::I32]);
		self.runtime_function(FLOAT_TEXT, vec![ValType::F64], vec![node_ref], locals, |s, f| {
			let (x, address, position, exponent, digits, significant, count) = (0, 1, 2, 3, 4, 5, 6);
			let return_text = |f: &mut Function, (text_address, length): (u32, u32)| {
				Self::emit_list(f, &[I::I32Const(text_address as i32), I::I32Const(length as i32)]);
				s.call(f, "new_text");
				f.instruction(&I::Return);
			};
			let put = |f: &mut Function, byte: u8| {
				Self::emit_list(f, &[I::LocalGet(position), I::I32Const(byte as i32), I::I32Store8(BYTE)]);
				Self::emit_list(f, &[I::LocalGet(position), I::I32Const(1), I::I32Add, I::LocalSet(position)]);
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
				Self::emit_list(f, &[I::LocalSet(count), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, &[I::LocalGet(count), I::I32Const(0), I::I32LeS, I::BrIf(1)]);
				put(f, b'0');
				Self::emit_list(f, &[I::LocalGet(count), I::I32Const(1), I::I32Sub, I::LocalSet(count), I::Br(0), I::End, I::End]);
			};

			Self::emit_list(f, &[I::LocalGet(x), I::LocalGet(x), I::F64Ne, I::If(BlockType::Empty)]);
			return_text(f, not_a_number);
			Self::emit_list(f, &[I::End, I::LocalGet(x), I::F64Const(f64::INFINITY.into()), I::F64Eq, I::If(BlockType::Empty)]);
			return_text(f, infinity);
			Self::emit_list(f, &[I::End, I::LocalGet(x), I::F64Const(f64::NEG_INFINITY.into()), I::F64Eq, I::If(BlockType::Empty)]);
			return_text(f, negative_infinity);
			Self::emit_list(f, &[I::End, I::LocalGet(x), I::F64Const(0.0.into()), I::F64Eq, I::If(BlockType::Empty)]);
			return_text(f, zero);
			f.instruction(&I::End);

			Self::emit_list(f, &[I::I32Const(TEXT_BYTES + SCRATCH_BYTES), I::LocalSet(count)]);
			s.emit_text_allocation(f, count, address);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalSet(position)]);
			Self::emit_list(f, &[I::LocalGet(x), I::F64Const(0.0.into()), I::F64Lt, I::If(BlockType::Empty)]);
			put(f, b'-');
			Self::emit_list(f, &[I::LocalGet(x), I::F64Neg, I::LocalSet(x), I::End]);

			// x = mantissa · 10^exponent with the mantissa in [1, 10)
			let step = |x_op: I<'static>, exponent_step: i32| [I::LocalGet(x), I::F64Const(10.0.into()), x_op, I::LocalSet(x), I::LocalGet(exponent), I::I32Const(exponent_step), I::I32Add, I::LocalSet(exponent)];
			Self::emit_while(f, &[I::LocalGet(x), I::F64Const(10.0.into()), I::F64Ge], &step(I::F64Div, 1));
			Self::emit_while(f, &[I::LocalGet(x), I::F64Const(1.0.into()), I::F64Lt], &step(I::F64Mul, -1));
			Self::emit_list(f, &[I::LocalGet(x), I::F64Const(MANTISSA_SCALE.into()), I::F64Mul, I::F64Nearest, I::I64TruncF64S, I::LocalTee(digits)]);
			// the mantissa is in [1e14, 1e15], so the trapping truncation is safe (trunc_sat needs a feature wasm-opt lacks)
			// 9.999…95 rounds up to 10^15: one digit more
			Self::emit_list(f, &[I::I64Const(MANTISSA_OVERFLOW), I::I64GeS, I::If(BlockType::Empty)]);
			Self::emit_list(f, &[I::LocalGet(digits), I::I64Const(10), I::I64DivS, I::LocalSet(digits)]);
			Self::emit_list(f, &[I::LocalGet(exponent), I::I32Const(1), I::I32Add, I::LocalSet(exponent), I::End]);
			Self::emit_list(f, &[I::I32Const(SIGNIFICANT_DIGITS), I::LocalSet(significant)]);
			let has_trailing_zero = [I::LocalGet(significant), I::I32Const(1), I::I32GtS, I::LocalGet(digits), I::I64Const(10), I::I64RemS, I::I64Eqz, I::I32And];
			let drop_trailing_zero = [I::LocalGet(digits), I::I64Const(10), I::I64DivS, I::LocalSet(digits), I::LocalGet(significant), I::I32Const(1), I::I32Sub, I::LocalSet(significant)];
			Self::emit_while(f, &has_trailing_zero, &drop_trailing_zero);
			scratch(f, &[I::I32Const(0)]);
			f.instruction(&I::LocalGet(digits));
			s.call(f, "int_to_decimal");
			f.instruction(&I::Drop);

			let all = [I::LocalGet(significant)];
			Self::emit_list(f, &[I::LocalGet(exponent), I::I32Const(SMALLEST_POSITIONAL), I::I32GeS, I::LocalGet(exponent), I::I32Const(LARGEST_POSITIONAL), I::I32LtS, I::I32And]);
			f.instruction(&I::If(BlockType::Empty));
			{
				// 1500: the digits, then zeros
				Self::emit_list(f, &[I::LocalGet(exponent), I::LocalGet(significant), I::I32Const(1), I::I32Sub, I::I32GeS, I::If(BlockType::Empty)]);
				copy(f, &[I::I32Const(0)], &all);
				zeros(f, &[I::LocalGet(exponent), I::LocalGet(significant), I::I32Sub, I::I32Const(1), I::I32Add]);
				// 1.5: the point after exponent+1 digits
				Self::emit_list(f, &[I::Else, I::LocalGet(exponent), I::I32Const(0), I::I32GeS, I::If(BlockType::Empty)]);
				let whole = [I::LocalGet(exponent), I::I32Const(1), I::I32Add];
				copy(f, &[I::I32Const(0)], &whole);
				put(f, b'.');
				copy(f, &whole, &[I::LocalGet(significant), I::LocalGet(exponent), I::I32Sub, I::I32Const(1), I::I32Sub]);
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
			Self::emit_list(f, &[I::LocalGet(position), I::LocalGet(position), I::LocalGet(exponent), I::I64ExtendI32S]);
			s.call(f, "int_to_decimal");
			Self::emit_list(f, &[I::I32Add, I::LocalSet(position), I::End]);

			Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(position), I::LocalGet(address), I::I32Sub]);
			s.call(f, "new_text");
		});
	}
}
