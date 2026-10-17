//! The $BigInt runtime behind the inline checks: sign-magnitude arithmetic, the heap of $BigInt behind Int handles, the slow (Int…) -> Int paths
use super::*;

impl WasmGcEmitter {
	/// Sign-magnitude $BigInt arithmetic
	pub(super) fn emit_signed_functions(&mut self) {
		let limbs = self.limbs_ref();
		let big = self.big_ref();
		let big_type = self.type_manager.big_int_type;
		let limbs_type = self.type_manager.limbs_type;

		// big_new(negative, limbs) normalizes: trimmed, zero never negative; locals: trimmed
		self.runtime_function("big_new", vec![ValType::I32, limbs], vec![big], vec![limbs], |s, f| {
			f.instruction(&I::LocalGet(1));
			s.call(f, "big_trim");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalTee(2), I::ArrayLen, I::I32Const(0), I::I32Ne, I::LocalGet(0), I::I32Const(0), I::I32Ne,
				I::I32And, I::LocalGet(2), I::StructNew(big_type),
			]);
		});

		// big_from_i64(x); locals: magnitude as u64
		self.runtime_function("big_from_i64", vec![ValType::I64], vec![big], vec![ValType::I64], |s, f| {
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I64Const(0), I::LocalGet(0), I::I64Sub, I::LocalGet(0), I::LocalGet(0), I::I64Const(0), I::I64LtS,
				I::Select, I::LocalSet(1),
				I::LocalGet(0), I::I64Const(0), I::I64LtS, I::LocalGet(1), I::I32WrapI64, I::LocalGet(1),
				I::I64Const(32), I::I64ShrU, I::I32WrapI64,
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

		// big_divmod(a, b) -> (quotient, remainder), truncated like i64.div_s / i64.rem_s; locals: their magnitudes
		let (quotient, remainder) = (2, 3);
		self.runtime_function("big_divmod", vec![big, big], vec![big, big], vec![limbs, limbs], |s, f| {
			s.magnitude(f, 1);
			Self::emit_list(f, &[I::ArrayLen, I::I32Eqz, I::If(BlockType::Empty), I::Unreachable, I::End]);
			s.magnitude(f, 0);
			s.magnitude(f, 1);
			s.call(f, "mag_divmod");
			Self::emit_list(f, &[I::LocalSet(remainder), I::LocalSet(quotient)]);
			s.negative(f, 0);
			s.negative(f, 1);
			Self::emit_list(f, &[I::I32Xor, I::LocalGet(quotient)]);
			s.call(f, "big_new");
			s.negative(f, 0);
			f.instruction(&I::LocalGet(remainder));
			s.call(f, "big_new");
		});

		// big_to_f64(a); locals: i, acc, magnitude
		self.runtime_function(
			"big_to_f64",
			vec![big],
			vec![ValType::F64],
			vec![ValType::I32, ValType::F64, limbs],
			|s, f| {
				s.magnitude(f, 0);
				Self::emit_list(f, &[I::LocalTee(3), I::ArrayLen, I::LocalSet(1)]);
				Self::count_down(f, 1, |f| {
					Self::emit_list(
						f,
						&[I::LocalGet(2), I::F64Const(LIMB_BASE.into()), I::F64Mul, I::LocalGet(3), I::LocalGet(1)],
					);
					s.limbs_get(f);
					Self::emit_list(f, &[I::F64ConvertI64U, I::F64Add, I::LocalSet(2)]);
				});
				Self::emit_list(f, &[I::LocalGet(2), I::F64Neg, I::LocalGet(2)]);
				s.negative(f, 0);
				f.instruction(&I::Select);
			},
		);

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
	pub(super) fn emit_handle_functions(&mut self) {
		let big = self.big_ref();
		let limbs = self.limbs_ref();
		let heap_type = self.type_manager.big_heap_type;
		let heap = Ref(RefType { nullable: true, heap_type: HeapType::Concrete(heap_type) });
		let (heap_global, count_global) = (self.int_heap_global, self.int_count_global);

		// int_store(b) -> new handle for a $BigInt or $Ratio; locals: heap, grown
		let any = Ref(RefType { nullable: true, heap_type: crate::type_kinds::any_heap_type() });
		self.runtime_function("int_store", vec![any], vec![ValType::I64], vec![heap, heap], |_, f| {
			let (stored, heap, grown) = (0, 1, 2);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::GlobalGet(heap_global), I::LocalTee(heap), I::RefIsNull, I::If(BlockType::Result(ValType::I32)),
					I::I32Const(1),
				I::Else,
					I::GlobalGet(count_global), I::LocalGet(heap), I::ArrayLen, I::I32GeU,
				I::End,
				I::If(BlockType::Empty),
					I::LocalGet(heap), I::RefIsNull, I::If(BlockType::Result(ValType::I32)),
						I::I32Const(INITIAL_HEAP_SIZE),
					I::Else,
						I::LocalGet(heap), I::ArrayLen, I::I32Const(1), I::I32Shl,
					I::End,
					I::ArrayNewDefault(heap_type), I::LocalSet(grown),
					I::LocalGet(heap), I::RefIsNull, I::I32Eqz, I::If(BlockType::Empty),
						I::LocalGet(grown), I::I32Const(0), I::LocalGet(heap), I::I32Const(0),
						I::GlobalGet(count_global),
						I::ArrayCopy { array_type_index_dst: heap_type, array_type_index_src: heap_type },
					I::End,
					I::LocalGet(grown), I::GlobalSet(heap_global),
					I::LocalGet(grown), I::LocalSet(heap),
				I::End,
				I::LocalGet(heap), I::GlobalGet(count_global), I::LocalGet(stored), I::ArraySet(heap_type),
				I::GlobalGet(count_global), I::I64ExtendI32U, I::I64Const(HANDLE_BASE), I::I64Add,
				I::GlobalGet(count_global), I::I32Const(1), I::I32Add, I::GlobalSet(count_global),
			]);
		});

		// int_box(b) -> fixnum when it fits, else a handle; locals: magnitude, value
		self.runtime_function("int_box", vec![big], vec![ValType::I64], vec![limbs, ValType::I64], |s, f| {
			let (big, magnitude, value) = (0, 1, 2);
			s.magnitude(f, 0);
			Self::emit_list(
				f,
				&[I::LocalTee(magnitude), I::ArrayLen, I::I32Const(2), I::I32LeU, I::If(BlockType::Empty)],
			);
			s.limb_or_zero(f, magnitude, I::I32Const(0));
			s.limb_or_zero(f, magnitude, I::I32Const(1));
			Self::emit_list(f, &[I::I64Const(32), I::I64Shl, I::I64Or, I::LocalSet(value)]);
			s.negative(f, 0);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::If(BlockType::Empty),
					I::LocalGet(value), I::I64Const(-(FIXNUM_MIN as i64)), I::I64LeU, I::If(BlockType::Empty),
						I::I64Const(0), I::LocalGet(value), I::I64Sub, I::Return,
					I::End,
				I::Else,
					I::LocalGet(value), I::I64Const(FIXNUM_MAX as i64), I::I64LeU, I::If(BlockType::Empty),
						I::LocalGet(value), I::Return,
				I::End, I::End,
				I::End,
				I::LocalGet(big),
			]);
			s.call(f, "int_store");
		});

		// int_unbox(x) -> $BigInt for any Int
		self.runtime_function("int_unbox", vec![ValType::I64], vec![big], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0]);
			Self::emit_list(f, &[I::If(BlockType::Result(big)), I::LocalGet(0)]);
			s.call(f, "big_from_i64");
			f.instruction(&I::Else);
			s.emit_heap_get_big(f, 0);
			f.instruction(&I::End);
		});
	}

	/// Slow paths behind the inline checks, all (Int...) -> Int
	pub(super) fn emit_int_functions(&mut self) {
		let (i64t, i32t, f64t) = (ValType::I64, ValType::I32, ValType::F64);
		let big = self.big_ref();
		let any = Ref(RefType { nullable: true, heap_type: crate::type_kinds::any_heap_type() });
		let i64_box = self.type_manager.i64_box_type;
		let big_type = self.type_manager.big_int_type;
		let ratio = self.type_manager.ratio_type;

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
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::If(BlockType::Empty),
					I::LocalGet(0), I::I64Eqz, I::If(BlockType::Empty),
						I::I64Const(0), I::Return,
					I::End,
					I::LocalGet(0), I::LocalGet(1), I::I64Mul, I::LocalTee(2), I::LocalGet(0), I::I64DivS,
					I::LocalGet(1), I::I64Eq,
			]);
			s.emit_fixnum_test(f, &[2]);
			Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty), I::LocalGet(2), I::Return, I::End, I::End]);
			unbox_both(s, f);
			s.call(f, "big_mul");
			s.call(f, "int_box");
		});

		// int_mul(a, b): checked multiply with the inline fast test, used by int_pow
		self.runtime_function("int_mul", vec![i64t, i64t], vec![i64t], vec![], |s, f| {
			Self::emit_mul_fast_test(f, &[0, 1]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::If(BlockType::Result(i64t)),
					I::LocalGet(0), I::LocalGet(1), I::I64Mul,
				I::Else,
					I::LocalGet(0), I::LocalGet(1),
			]);
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
			Self::emit_list(f, &[I::LocalSet(1), I::I32Const(0)]);
			s.magnitude(f, 1);
			f.instruction(&I::StructNew(big_type));
			s.call(f, "int_box");
		});

		// int_cmp(a, b) -> -1/0/1
		self.runtime_function("int_cmp", vec![i64t, i64t], vec![i32t], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0, 1]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::If(BlockType::Result(i32t)),
					I::LocalGet(0), I::LocalGet(1), I::I64GtS, I::LocalGet(0), I::LocalGet(1), I::I64LtS, I::I32Sub,
				I::Else,
			]);
			unbox_both(s, f);
			s.call(f, "big_cmp");
			f.instruction(&I::End);
		});

		// int_divmod_slow(a, b) -> (quotient, remainder): one long division yields both; locals: boxed remainder
		self.runtime_function("int_divmod_slow", vec![i64t, i64t], vec![i64t, i64t], vec![big], |s, f| {
			unbox_both(s, f);
			s.call(f, "big_divmod");
			f.instruction(&I::LocalSet(2));
			s.call(f, "int_box");
			f.instruction(&I::LocalGet(2));
			s.call(f, "int_box");
		});
		// int_quot_slow / int_rem_slow: truncated division, the other result dropped; locals: remainder
		for (name, keep_wanted) in
			[("int_quot_slow", &[I::Drop][..]), ("int_rem_slow", &[I::LocalSet(2), I::Drop, I::LocalGet(2)])]
		{
			self.runtime_function(name, vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
				Self::emit_list(f, &[I::LocalGet(0), I::LocalGet(1)]);
				s.call(f, "int_divmod_slow");
				Self::emit_list(f, keep_wanted);
			});
		}

		// a fixnum converts as it is, rounded once like big_to_f64; only a handle's $BigInt is read limb by limb
		self.runtime_function("int_to_f64", vec![i64t], vec![f64t], vec![], |s, f| {
			s.emit_fixnum_test(f, &[0]);
			Self::emit_list(f, &[I::If(BlockType::Result(f64t)), I::LocalGet(0), I::F64ConvertI64S, I::Else]);
			s.emit_heap_get_big(f, 0);
			s.call(f, "big_to_f64");
			f.instruction(&I::End);
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

		// int_pow(base, exponent) by squaring; negative or huge exponents trap; locals: result
		self.runtime_function("int_pow", vec![i64t, i64t], vec![i64t], vec![i64t], |s, f| {
			let (base, exponent, result) = (0, 1, 2);
			Self::emit_list(
				f,
				&[I::LocalGet(exponent), I::I64Const(0), I::I64LtS, I::If(BlockType::Empty), I::Unreachable, I::End],
			);
			s.emit_fixnum_test(f, &[1]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::I32Eqz, I::If(BlockType::Empty),
					I::Unreachable,
				I::End,
				I::I64Const(1), I::LocalSet(result),
				I::Block(BlockType::Empty),
					I::Loop(BlockType::Empty),
						I::LocalGet(exponent), I::I64Eqz, I::BrIf(1),
						I::LocalGet(exponent), I::I64Const(1), I::I64And, I::I32WrapI64, I::If(BlockType::Empty),
							I::LocalGet(result), I::LocalGet(base),
			]);
			s.call(f, "int_mul");
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(result),
				I::End,
				I::LocalGet(exponent), I::I64Const(1), I::I64ShrS, I::LocalTee(exponent), I::I64Eqz, I::BrIf(1),
				I::LocalGet(base), I::LocalGet(base),
			]);
			s.call(f, "int_mul");
			Self::emit_list(f, &[I::LocalSet(base), I::Br(0), I::End, I::End, I::LocalGet(result)]);
		});

		// int_from_payload(data) -> Int for the data field of an Int node
		self.runtime_function("int_from_payload", vec![any], vec![i64t], vec![], |s, f| {
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(0), I::RefTestNonNull(HeapType::Concrete(i64_box)), I::If(BlockType::Result(i64t)),
					I::LocalGet(0), I::RefCastNonNull(HeapType::Concrete(i64_box)),
					I::StructGet { struct_type_index: i64_box, field_index: 0 },
				I::Else,
					I::LocalGet(0), I::RefTestNonNull(HeapType::Concrete(ratio)), I::If(BlockType::Result(i64t)),
						I::LocalGet(0),
			]);
			s.call(f, "int_store");
			Self::emit_list(f, &[I::Else, I::LocalGet(0), I::RefCastNonNull(HeapType::Concrete(big_type))]);
			s.call(f, "int_box");
			Self::emit_list(f, &[I::End, I::End]);
		});
	}
}
