//! Unsigned limb-array arithmetic: the magnitude functions and division (bitwise and limb by limb)
use super::*;

impl WasmGcEmitter {
	/// Unsigned limb-array arithmetic
	pub(super) fn emit_magnitude_functions(&mut self) {
		let limbs = self.limbs_ref();
		let limbs_type = self.type_manager.limbs_type;

		// big_trim(m) -> m without leading zero limbs; locals: n, trimmed
		self.runtime_function("big_trim", vec![limbs], vec![limbs], vec![ValType::I32, limbs], |_, f| {
			let (magnitude, n, trimmed) = (0, 1, 2);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(magnitude), I::ArrayLen, I::LocalSet(n),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(n), I::I32Eqz, I::BrIf(1),
						I::LocalGet(magnitude), I::LocalGet(n), I::I32Const(1), I::I32Sub, I::ArrayGet(limbs_type),
						I::BrIf(1),
			]);
			Self::increment(f, n, -1);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::Br(0),
				I::End,
				I::End,
				I::LocalGet(n), I::LocalGet(magnitude), I::ArrayLen, I::I32Eq, I::If(BlockType::Result(limbs)),
					I::LocalGet(magnitude),
				I::Else,
					I::LocalGet(n), I::ArrayNewDefault(limbs_type), I::LocalSet(trimmed),
					I::LocalGet(trimmed), I::I32Const(0), I::LocalGet(magnitude), I::I32Const(0), I::LocalGet(n),
					I::ArrayCopy { array_type_index_dst: limbs_type, array_type_index_src: limbs_type },
					I::LocalGet(trimmed),
				I::End,
			]);
		});

		// mag_cmp(a, b) -> -1/0/1; locals: i, x, y
		self.runtime_function(
			"mag_cmp",
			vec![limbs, limbs],
			vec![ValType::I32],
			vec![ValType::I32, ValType::I64, ValType::I64],
			|s, f| {
				let (a, b, i, x, y) = (0, 1, 2, 3, 4);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(a), I::ArrayLen, I::LocalGet(b), I::ArrayLen, I::LocalGet(a), I::ArrayLen,
					I::LocalGet(b), I::ArrayLen, I::I32GtU, I::Select, I::LocalSet(i),
				]);
				Self::count_down(f, i, |f| {
					s.limb_or_zero(f, a, I::LocalGet(i));
					f.instruction(&I::LocalSet(x));
					s.limb_or_zero(f, b, I::LocalGet(i));
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalSet(y),
						I::LocalGet(x), I::LocalGet(y), I::I64Ne, I::If(BlockType::Empty),
							I::I32Const(1), I::I32Const(-1), I::LocalGet(x), I::LocalGet(y), I::I64GtU, I::Select,
							I::Return,
						I::End,
					]);
				});
				f.instruction(&I::I32Const(0));
			},
		);

		// mag_add(a, b); locals: n, sum, i, acc
		self.runtime_function(
			"mag_add",
			vec![limbs, limbs],
			vec![limbs],
			vec![ValType::I32, limbs, ValType::I32, ValType::I64],
			|s, f| {
				let (a, b, n, sum, i, acc) = (0, 1, 2, 3, 4, 5);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(a), I::ArrayLen, I::LocalGet(b), I::ArrayLen, I::LocalGet(a), I::ArrayLen,
					I::LocalGet(b), I::ArrayLen, I::I32GtU, I::Select, I::LocalSet(n),
					I::LocalGet(n), I::I32Const(1), I::I32Add, I::ArrayNewDefault(limbs_type), I::LocalSet(sum),
				]);
				Self::count_up(f, i, n, |f| {
					s.limb_or_zero(f, a, I::LocalGet(i));
					s.limb_or_zero(f, b, I::LocalGet(i));
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I64Add, I::LocalGet(acc), I::I64Add, I::LocalSet(acc),
						I::LocalGet(sum), I::LocalGet(i), I::LocalGet(acc), I::I32WrapI64,
					]);
					s.limbs_set(f);
					Self::emit_list(f, &[I::LocalGet(acc), I::I64Const(32), I::I64ShrU, I::LocalSet(acc)]);
				});
				Self::emit_list(f, &[I::LocalGet(sum), I::LocalGet(n), I::LocalGet(acc), I::I32WrapI64]);
				s.limbs_set(f);
				f.instruction(&I::LocalGet(sum));
				s.call(f, "big_trim");
			},
		);

		// mag_sub_into(r, b): r -= b in place, requires r >= b; locals: n, i, acc, borrow
		self.runtime_function(
			"mag_sub_into",
			vec![limbs, limbs],
			vec![],
			vec![ValType::I32, ValType::I32, ValType::I64, ValType::I64],
			|s, f| {
				let (r, b, n, i, acc, borrow) = (0, 1, 2, 3, 4, 5);
				Self::emit_list(f, &[I::LocalGet(r), I::ArrayLen, I::LocalSet(n)]);
				Self::count_up(f, i, n, |f| {
					s.limb_or_zero(f, r, I::LocalGet(i));
					s.limb_or_zero(f, b, I::LocalGet(i));
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::I64Sub, I::LocalGet(borrow), I::I64Sub, I::LocalSet(acc),
						I::LocalGet(acc), I::I64Const(0), I::I64LtS, I::I64ExtendI32U, I::LocalSet(borrow),
						I::LocalGet(r), I::LocalGet(i), I::LocalGet(acc), I::I32WrapI64,
					]);
					s.limbs_set(f);
				});
			},
		);

		// mag_sub(a, b) -> a - b, requires a >= b; locals: copy
		self.runtime_function("mag_sub", vec![limbs, limbs], vec![limbs], vec![limbs], |s, f| {
			let (a, b, copy) = (0, 1, 2);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(a), I::ArrayLen, I::ArrayNewDefault(limbs_type), I::LocalSet(copy),
				I::LocalGet(copy), I::I32Const(0), I::LocalGet(a), I::I32Const(0), I::LocalGet(a), I::ArrayLen,
				I::ArrayCopy { array_type_index_dst: limbs_type, array_type_index_src: limbs_type },
				I::LocalGet(copy), I::LocalGet(b),
			]);
			s.call(f, "mag_sub_into");
			f.instruction(&I::LocalGet(copy));
			s.call(f, "big_trim");
		});

		// mag_mul(a, b) schoolbook; locals: la, lb, product, i, j, ai, carry
		let i32s = ValType::I32;
		self.runtime_function(
			"mag_mul",
			vec![limbs, limbs],
			vec![limbs],
			vec![i32s, i32s, limbs, i32s, i32s, ValType::I64, ValType::I64],
			|s, f| {
				let (a, b, la, lb, product, i, j, ai, carry) = (0, 1, 2, 3, 4, 5, 6, 7, 8);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(a), I::ArrayLen, I::LocalSet(la),
					I::LocalGet(b), I::ArrayLen, I::LocalSet(lb),
					I::LocalGet(la), I::LocalGet(lb), I::I32Add, I::ArrayNewDefault(limbs_type), I::LocalSet(product),
				]);
				Self::count_up(f, i, la, |f| {
					Self::emit_list(f, &[I::LocalGet(a), I::LocalGet(i)]);
					s.limbs_get(f);
					Self::emit_list(
						f,
						&[I::LocalSet(ai), I::I64Const(0), I::LocalSet(carry), I::I32Const(0), I::LocalSet(j)],
					);
					Self::count_up(f, j, lb, |f| {
						// t = ai * b[j] + product[i+j] + carry  (< 2^64)
						Self::emit_list(f, &[I::LocalGet(ai), I::LocalGet(b), I::LocalGet(j)]);
						s.limbs_get(f);
						Self::emit_list(
							f,
							&[I::I64Mul, I::LocalGet(product), I::LocalGet(i), I::LocalGet(j), I::I32Add],
						);
						s.limbs_get(f);
						#[rustfmt::skip]
						Self::emit_list(f, &[
							I::I64Add, I::LocalGet(carry), I::I64Add, I::LocalSet(carry),
							I::LocalGet(product), I::LocalGet(i), I::LocalGet(j), I::I32Add, I::LocalGet(carry),
							I::I32WrapI64,
						]);
						s.limbs_set(f);
						Self::emit_list(f, &[I::LocalGet(carry), I::I64Const(32), I::I64ShrU, I::LocalSet(carry)]);
					});
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(product), I::LocalGet(i), I::LocalGet(lb), I::I32Add, I::LocalGet(carry),
						I::I32WrapI64,
					]);
					s.limbs_set(f);
				});
				f.instruction(&I::LocalGet(product));
				s.call(f, "big_trim");
			},
		);

		// mag_divmod(a, b) -> (quotient, remainder) as two results, b nonzero: limb by limb where ratios need gcds, else the
		// compact bit by bit division (web::test_bundle_budget)
		if self.should_emit_function(super::super::super::numbers::exact::INT_GCD) {
			self.emit_limbwise_divmod();
		} else {
			self.emit_bitwise_divmod();
		}
	}

	/// mag_divmod by binary long division: O(bits a · len b), compact
	pub(super) fn emit_bitwise_divmod(&mut self) {
		let (limbs, limbs_type, i32s) = (self.limbs_ref(), self.type_manager.limbs_type, ValType::I32);
		// locals: quotient, remainder, bit, n, k, carry, v
		self.runtime_function(
			"mag_divmod",
			vec![limbs, limbs],
			vec![limbs, limbs],
			vec![limbs, limbs, i32s, i32s, i32s, i32s, i32s],
			|s, f| {
				let (a, b, quotient, remainder, bit, n, k, carry, v) = (0, 1, 2, 3, 4, 5, 6, 7, 8);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::LocalGet(a), I::ArrayLen, I::ArrayNewDefault(limbs_type), I::LocalSet(quotient),
					I::LocalGet(b), I::ArrayLen, I::I32Const(1), I::I32Add, I::LocalTee(n),
					I::ArrayNewDefault(limbs_type), I::LocalSet(remainder),
					I::LocalGet(a), I::ArrayLen, I::I32Const(5), I::I32Shl, I::LocalSet(bit),
				]);
				Self::count_down(f, bit, |f| {
					// carry = bit `bit` of a
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(a), I::LocalGet(bit), I::I32Const(5), I::I32ShrU, I::ArrayGet(limbs_type),
						I::LocalGet(bit), I::I32Const(31), I::I32And, I::I32ShrU, I::I32Const(1), I::I32And,
						I::LocalSet(carry),
					]);
					// remainder = remainder << 1 | carry
					Self::emit_list(f, &[I::I32Const(0), I::LocalSet(k)]);
					Self::count_up(f, k, n, |f| {
						#[rustfmt::skip]
						Self::emit_list(f, &[
							I::LocalGet(remainder), I::LocalGet(k), I::ArrayGet(limbs_type), I::LocalSet(v),
							I::LocalGet(remainder), I::LocalGet(k), I::LocalGet(v), I::I32Const(1), I::I32Shl,
							I::LocalGet(carry), I::I32Or, I::ArraySet(limbs_type),
							I::LocalGet(v), I::I32Const(31), I::I32ShrU, I::LocalSet(carry),
						]);
					});
					Self::emit_list(f, &[I::LocalGet(remainder), I::LocalGet(b)]);
					s.call(f, "mag_cmp");
					Self::emit_list(
						f,
						&[I::I32Const(0), I::I32GeS, I::If(BlockType::Empty), I::LocalGet(remainder), I::LocalGet(b)],
					);
					s.call(f, "mag_sub_into");
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(quotient), I::LocalGet(bit), I::I32Const(5), I::I32ShrU, I::LocalGet(quotient),
						I::LocalGet(bit), I::I32Const(5), I::I32ShrU, I::ArrayGet(limbs_type), I::I32Const(1),
						I::LocalGet(bit), I::I32Const(31), I::I32And, I::I32Shl, I::I32Or, I::ArraySet(limbs_type),
						I::End,
					]);
				});
				f.instruction(&I::LocalGet(quotient));
				s.call(f, "big_trim");
				f.instruction(&I::LocalGet(remainder));
				s.call(f, "big_trim");
			},
		);
	}

	/// mag_divmod by long division limb by limb (Knuth, TAOCP 4.3.1 algorithm D): O(len a · len b); bit by bit, Euclid's
	/// gcd of 1400-bit numbers ran out of fuel (card exact-harmonic)
	pub(super) fn emit_limbwise_divmod(&mut self) {
		let (limbs, limbs_type, i32s) = (self.limbs_ref(), self.type_manager.limbs_type, ValType::I32);
		let i64s = ValType::I64;
		let locals = vec![
			limbs, limbs, limbs, i32s, i32s, i32s, i32s, i32s, i32s, i64s, i64s, i64s, i64s, i64s, i64s, i64s, i32s,
			i32s,
		];
		self.runtime_function("mag_divmod", vec![limbs, limbs], vec![limbs, limbs], locals, |s, f| {
			let (a, b, quotient, un, vn, la, lb, shift, j, i, bound) = (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
			let (qhat, rhat, product, t, k, v_top, v_second, previous, top) = (11, 12, 13, 14, 15, 16, 17, 18, 19);
			let get = |f: &mut Function, array: u32, index: &[Instruction]| {
				f.instruction(&I::LocalGet(array));
				Self::emit_list(f, index);
				s.limbs_get(f);
			};
			let set = |f: &mut Function, array: u32, index: &[Instruction], value: &[Instruction]| {
				f.instruction(&I::LocalGet(array));
				Self::emit_list(f, index);
				Self::emit_list(f, value);
				f.instruction(&I::I32WrapI64);
				s.limbs_set(f);
			};
			let new_limbs = |f: &mut Function, length: &[Instruction], local: u32| {
				Self::emit_list(f, length);
				Self::emit_list(f, &[I::ArrayNewDefault(limbs_type), I::LocalSet(local)]);
			};
			let range = |f: &mut Function, length: &[Instruction], body: &dyn Fn(&mut Function)| {
				Self::emit_list(f, &[I::I32Const(0), I::LocalSet(i)]);
				Self::emit_list(f, length);
				f.instruction(&I::LocalSet(bound));
				Self::count_up(f, i, bound, body);
			};
			// dst = src << shift, limb by limb; the bits shifted out of the top stay in k
			let shift_into = |f: &mut Function, source: u32, destination: u32, length: u32| {
				Self::emit_list(f, &[I::I64Const(0), I::LocalSet(k)]);
				range(f, &[I::LocalGet(length)], &|f| {
					get(f, source, &[I::LocalGet(i)]);
					Self::emit_list(
						f,
						&[I::LocalGet(shift), I::I64ExtendI32U, I::I64Shl, I::LocalGet(k), I::I64Or, I::LocalSet(t)],
					);
					set(f, destination, &[I::LocalGet(i)], &[I::LocalGet(t)]);
					Self::emit_list(f, &[I::LocalGet(t), I::I64Const(32), I::I64ShrU, I::LocalSet(k)]);
				});
			};
			for local in [a, b] {
				f.instruction(&I::LocalGet(local));
				s.call(f, "big_trim");
				f.instruction(&I::LocalSet(local));
			}
			Self::emit_list(
				f,
				&[I::LocalGet(a), I::ArrayLen, I::LocalSet(la), I::LocalGet(b), I::ArrayLen, I::LocalSet(lb)],
			);
			// a < b: quotient 0, remainder a
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(la), I::LocalGet(lb), I::I32LtU, I::If(BlockType::Empty),
					I::I32Const(0), I::ArrayNewDefault(limbs_type), I::LocalGet(a), I::Return,
				I::End,
			]);
			new_limbs(f, &[I::LocalGet(la), I::LocalGet(lb), I::I32Sub, I::I32Const(1), I::I32Add], quotient);
			// normalize: shift both left until the divisor's top bit is set
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(b), I::LocalGet(lb), I::I32Const(1), I::I32Sub, I::ArrayGet(limbs_type), I::I32Clz,
				I::LocalSet(shift),
			]);
			new_limbs(f, &[I::LocalGet(lb)], vn);
			shift_into(f, b, vn, lb);
			new_limbs(f, &[I::LocalGet(la), I::I32Const(1), I::I32Add], un);
			shift_into(f, a, un, la);
			set(f, un, &[I::LocalGet(la)], &[I::LocalGet(k)]);
			get(f, vn, &[I::LocalGet(lb), I::I32Const(1), I::I32Sub]);
			f.instruction(&I::LocalSet(v_top));
			// a one-limb divisor has no second limb: its estimates are exact
			Self::emit_list(f, &[I::LocalGet(lb), I::I32Const(2), I::I32Sub, I::LocalSet(previous)]);
			s.limb_or_zero(f, vn, I::LocalGet(previous));
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(v_second),
				I::LocalGet(la), I::LocalGet(lb), I::I32Sub, I::I32Const(1), I::I32Add, I::LocalSet(j),
			]);
			Self::count_down(f, j, |f| {
				Self::emit_list(f, &[I::LocalGet(j), I::LocalGet(lb), I::I32Add, I::LocalSet(top)]);
				// estimate the quotient limb from the top two limbs, at most 2 too big
				get(f, un, &[I::LocalGet(top)]);
				Self::emit_list(f, &[I::I64Const(32), I::I64Shl]);
				get(f, un, &[I::LocalGet(top), I::I32Const(1), I::I32Sub]);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Or, I::LocalTee(t), I::LocalGet(v_top), I::I64DivU, I::LocalSet(qhat),
					I::LocalGet(t), I::LocalGet(v_top), I::I64RemU, I::LocalSet(rhat),
					I::Block(BlockType::Empty),
						I::Loop(BlockType::Empty),
							I::LocalGet(qhat), I::I64Const(1 << 32), I::I64GeU, I::If(BlockType::Result(ValType::I32)),
								I::I32Const(1),
							I::Else,
								I::LocalGet(qhat), I::LocalGet(v_second), I::I64Mul, I::LocalGet(rhat), I::I64Const(32),
								I::I64Shl, I::LocalGet(top), I::I32Const(2), I::I32Sub, I::LocalSet(previous),
				]);
				s.limb_or_zero(f, un, I::LocalGet(previous));
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::I64Or, I::I64GtU,
					I::End,
					I::I32Eqz, I::BrIf(1),
					I::LocalGet(qhat), I::I64Const(1), I::I64Sub, I::LocalSet(qhat),
					I::LocalGet(rhat), I::LocalGet(v_top), I::I64Add, I::LocalTee(rhat), I::I64Const(1 << 32),
					I::I64LtU, I::BrIf(0),
					I::End,
					I::End,
				]);
				// un[j ..= j+lb] -= qhat · vn
				Self::emit_list(f, &[I::I64Const(0), I::LocalSet(k)]);
				range(f, &[I::LocalGet(lb)], &|f| {
					f.instruction(&I::LocalGet(qhat));
					get(f, vn, &[I::LocalGet(i)]);
					Self::emit_list(f, &[I::I64Mul, I::LocalSet(product)]);
					get(f, un, &[I::LocalGet(i), I::LocalGet(j), I::I32Add]);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(k), I::I64Sub, I::LocalGet(product), I::I64Const(0xFFFF_FFFF), I::I64And, I::I64Sub,
						I::LocalSet(t),
					]);
					set(f, un, &[I::LocalGet(i), I::LocalGet(j), I::I32Add], &[I::LocalGet(t)]);
					#[rustfmt::skip]
					Self::emit_list(f, &[
						I::LocalGet(product), I::I64Const(32), I::I64ShrU, I::LocalGet(t), I::I64Const(32), I::I64ShrS,
						I::I64Sub, I::LocalSet(k),
					]);
				});
				get(f, un, &[I::LocalGet(top)]);
				Self::emit_list(f, &[I::LocalGet(k), I::I64Sub, I::LocalSet(t)]);
				set(f, un, &[I::LocalGet(top)], &[I::LocalGet(t)]);
				set(f, quotient, &[I::LocalGet(j)], &[I::LocalGet(qhat)]);
				// subtracted once too often: add the divisor back
				Self::emit_list(f, &[I::LocalGet(t), I::I64Const(0), I::I64LtS, I::If(BlockType::Empty)]);
				set(f, quotient, &[I::LocalGet(j)], &[I::LocalGet(qhat), I::I64Const(1), I::I64Sub]);
				Self::emit_list(f, &[I::I64Const(0), I::LocalSet(k)]);
				range(f, &[I::LocalGet(lb)], &|f| {
					get(f, un, &[I::LocalGet(i), I::LocalGet(j), I::I32Add]);
					get(f, vn, &[I::LocalGet(i)]);
					Self::emit_list(f, &[I::I64Add, I::LocalGet(k), I::I64Add, I::LocalSet(t)]);
					set(f, un, &[I::LocalGet(i), I::LocalGet(j), I::I32Add], &[I::LocalGet(t)]);
					Self::emit_list(f, &[I::LocalGet(t), I::I64Const(32), I::I64ShrU, I::LocalSet(k)]);
				});
				get(f, un, &[I::LocalGet(top)]);
				Self::emit_list(f, &[I::LocalGet(k), I::I64Add, I::LocalSet(t)]);
				set(f, un, &[I::LocalGet(top)], &[I::LocalGet(t)]);
				f.instruction(&I::End);
			});
			// the remainder: the low limbs of un shifted back down
			new_limbs(f, &[I::LocalGet(lb)], vn);
			range(f, &[I::LocalGet(lb)], &|f| {
				Self::emit_list(f, &[I::LocalGet(vn), I::LocalGet(i)]);
				get(f, un, &[I::LocalGet(i), I::I32Const(1), I::I32Add]);
				Self::emit_list(f, &[I::I64Const(32), I::I64Shl]);
				get(f, un, &[I::LocalGet(i)]);
				Self::emit_list(f, &[I::I64Or, I::LocalGet(shift), I::I64ExtendI32U, I::I64ShrU, I::I32WrapI64]);
				s.limbs_set(f);
			});
			f.instruction(&I::LocalGet(quotient));
			s.call(f, "big_trim");
			f.instruction(&I::LocalGet(vn));
			s.call(f, "big_trim");
		});
	}
}
