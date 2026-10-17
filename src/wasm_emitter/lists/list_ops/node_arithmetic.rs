//! Arithmetic on Nodes whose kind only the run time knows, and texts as numbers
use super::*;
use crate::wasm_emitter::runtime_dependencies::{any, Dependency};
use crate::wasm_emitter::INT_RUNTIME;

/// text_as_int(node) -> i64: a Text's optional sign and decimal digits, any other node's Int (get_int_value)
pub const TEXT_AS_INT: &str = "text_as_int";
/// Arithmetic on two Nodes of run-time kind: (function, its operator, the exact Int function); the first four in
/// uncertain.rs UNCERTAIN_ARITHMETIC's order
pub const NODE_ADD: &str = "node_add";
pub const NODE_POW: &str = "node_pow";
pub const NODE_ARITHMETIC: [(&str, Op, &str); 7] = [
	(NODE_ADD, Op::Add, "exact_add"),
	("node_sub", Op::Sub, "exact_sub"),
	("node_mul", Op::Mul, "exact_mul"),
	("node_div", Op::Div, "exact_div"),
	("node_mod", Op::Mod, "exact_mod"),
	("node_rem", Op::Rem, "exact_rem"),
	(NODE_POW, Op::Pow, "exact_pow"),
];

/// The node arithmetic reads texts as numbers and falls back to exact Ints; each function needs its exact one
pub(in crate::wasm_emitter) fn node_arithmetic_dependencies() -> Vec<Dependency> {
	let functions: Vec<&'static str> = NODE_ARITHMETIC.iter().map(|(name, _, _)| *name).collect();
	let shared =
		any(&functions, &[TEXT_AS_FLOAT, INT_RUNTIME, "exact_add", "exact_sub", "exact_mul", "exact_div", "new_float"]);
	let exact = NODE_ARITHMETIC.iter().map(|(name, _, exact)| any(&[*name], &[*exact]));
	[shared, any(&[NODE_POW], &["is_ratio", "invalid_number"])].into_iter().chain(exact).collect()
}

/// text_as_float(node): the f64 of a text like "-12.5e3" (a number node converts as it is); anything else is invalid_number
pub const TEXT_AS_FLOAT: &str = "text_as_float";
/// 10^22 is the largest power of ten an f64 holds exactly
const EXACT_POWERS_OF_TEN: i32 = 22;
/// The educate-once topic of `"ab"*2`, whose explicit form is `2 times "ab"`
pub(super) const TEXT_REPEAT_TOPIC: &str = "text-repeat";

impl WasmGcEmitter {
	/// A leading sign of the text at `pointer`: `negative` set for -, the pointer past a - or + (`digit` holds the byte)
	fn emit_skip_sign(f: &mut Function, pointer: u32, digit: u32, negative: u32) {
		#[rustfmt::skip]
		Self::emit_list(f, &[
			I::LocalGet(pointer), I::I32Load8U(BYTE), I::LocalTee(digit), I32Const('-' as i32), I::I32Eq,
			I::LocalSet(negative),
			I::LocalGet(negative), I::LocalGet(digit), I32Const('+' as i32), I::I32Eq, I::I32Or, I::LocalGet(pointer),
			I::I32Add, I::LocalSet(pointer),
		]);
	}

	/// `x as int` of a text held in a variable, like the literal `"12" as int`; no digits is the runtime error invalid_number
	pub(in crate::wasm_emitter) fn emit_text_as_int(&mut self) {
		if !self.should_emit_function(TEXT_AS_INT) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let locals = vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I64];
		self.runtime_function(TEXT_AS_INT, vec![node_ref], vec![ValType::I64], locals, |s, f| {
			let (pointer, end, negative, digit, value) = (1, 2, 3, 4, 5);
			let at_end = [I::LocalGet(pointer), I::LocalGet(end), I::I32GeU];
			// a one-character text is a character node: its digit, else invalid_number
			s.emit_field(f, 0, 0);
			Self::emit_kind_test(f, Kind::Codepoint as i64, I::I64Eq);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(0)]);
			s.emit_codepoint_of_node(f);
			Self::emit_list(f, &[I::I64Const('0' as i64), I::I64Sub, I::LocalTee(value), I::I64Const(9), I::I64GtU]);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &[I::LocalGet(value), I::Return, I::End]);
			// a float truncates toward zero (`(x*x) as int` of a float parameter held as a Node)
			let (node_type, float_box) = (s.type_manager.node_type, s.type_manager.f64_box_type);
			s.emit_field(f, 0, 0);
			Self::emit_kind_test(f, Kind::Float as i64, I::I64Eq);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::If(BlockType::Empty),
					I::LocalGet(0), I::StructGet { struct_type_index: node_type, field_index: 1 },
					I::RefCastNonNull(HeapType::Concrete(float_box)),
					I::StructGet { struct_type_index: float_box, field_index: 0 },
			]);
			// a checked truncation: wasm-split and wasm-opt reject the saturating one without nontrapping-float-to-int
			s.emit_truncating_cast_via(f, value);
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_field(f, 0, 0);
			Self::emit_kind_test(f, Kind::Text as i64, I::I64Ne);
			Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(0)]);
			s.call(f, "get_int_value");
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_text_bounds(f, pointer, end);
			Self::emit_list(f, &at_end);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_skip_sign(f, pointer, digit, negative);
			Self::emit_list(f, &at_end);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
			Self::emit_list(f, &at_end);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::BrIf(1),
				I::LocalGet(pointer), I::I32Load8U(BYTE), I32Const('0' as i32), I::I32Sub, I::LocalTee(digit),
				I32Const(9), I::I32GtU,
			]);
			s.emit_fail_if(f, "invalid_number");
			// value = value*10 + digit, unbounded with the big-int runtime ("99999999999999999999" as int)
			let unbounded = s.int_runtime();
			Self::emit_list(f, &[I::LocalGet(value), I::I64Const(10)]);
			if unbounded {
				s.call(f, "exact_mul")
			} else {
				f.instruction(&I::I64Mul);
			}
			Self::emit_list(f, &[I::LocalGet(digit), I::I64ExtendI32U]);
			if unbounded {
				s.call(f, "exact_add")
			} else {
				f.instruction(&I::I64Add);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalSet(value),
				I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalSet(pointer),
				I::Br(0),
				I::End,
				I::End,
				I::I64Const(0), I::LocalGet(value),
			]);
			if unbounded {
				s.call(f, "exact_sub")
			} else {
				f.instruction(&I::I64Sub);
			}
			Self::emit_list(f, &[I::LocalGet(value), I::LocalGet(negative), I::Select]);
		});
	}

	/// The f64 of `a op b` of the Nodes in params 0 and 1; a remainder reads them again instead of holding them in the
	/// emitter's scratch locals, which a runtime function has not (arithmetic.rs emit_float_remainder)
	pub(super) fn emit_node_float_arithmetic(&mut self, f: &mut Function, op: Op, power: u32) {
		let operand = |s: &mut Self, f: &mut Function, param: u32, absolute: bool| {
			f.instruction(&I::LocalGet(param));
			s.call(f, TEXT_AS_FLOAT);
			if absolute {
				f.instruction(&I::F64Abs);
			}
		};
		let (dividend, divisor) = (0, 1);
		// libm's pow; a NaN (a negative base with a fractional exponent) is invalid_number, as emit_float_power makes it
		if op == Op::Pow {
			operand(self, f, dividend, false);
			operand(self, f, divisor, false);
			if self.emit_libm_call(f, crate::wasm_emitter::numbers::arithmetic::LIBM_POW) {
				Self::emit_list(f, &[I::LocalTee(power), I::LocalGet(power), I::F64Ne]);
				self.emit_fail_if(f, "invalid_number");
				f.instruction(&I::LocalGet(power));
			}
			return;
		}
		if !matches!(op, Op::Mod | Op::Rem) {
			operand(self, f, dividend, false);
			operand(self, f, divisor, false);
			return self.emit_float_arithmetic(f, &op);
		}
		// `a - |b| * floor(a / |b|)` (euclidean) or `a - b * trunc(a / b)`
		let euclidean = op == Op::Mod;
		operand(self, f, dividend, false);
		operand(self, f, dividend, false);
		operand(self, f, divisor, euclidean);
		f.instruction(&I::F64Div);
		f.instruction(if euclidean { &I::F64Floor } else { &I::F64Trunc });
		operand(self, f, divisor, euclidean);
		Self::emit_list(f, &[I::F64Mul, I::F64Sub]);
	}

	/// node_add(a, b) … node_rem(a, b): arithmetic on two values whose kinds are known only at run time (fields of a parsed
	/// JSON map, elements of a list parameter): Floats when either is a Float, else exact Ints; anything else is the
	/// runtime error not_an_int
	pub(in crate::wasm_emitter) fn emit_node_arithmetic(&mut self) {
		let node_ref = Ref(self.node_ref(false));
		let (int_kind, float_kind) = (crate::type_kinds::Kind::Int as i64, crate::type_kinds::Kind::Float as i64);
		for (index, (name, op, exact_op)) in NODE_ARITHMETIC.into_iter().enumerate() {
			if !self.should_emit_function(name) {
				continue;
			}
			self.runtime_function(
				name,
				vec![node_ref, node_ref],
				vec![node_ref],
				vec![ValType::I64, ValType::F64],
				|s, f| {
					let (kind, power) = (2, 3);
					// a time or duration: calendar arithmetic (time_arithmetic.rs)
					if let Some(operator) = super::super::super::time::time_arithmetic::TIME_OPERATORS
						.iter()
						.position(|known| *known == op)
						.filter(|_| s.uses_times())
					{
						for operand in [0, 1] {
							s.emit_field(f, operand, 0);
							Self::emit_kind_test(f, Kind::Time as i64, I::I64Eq);
						}
						#[rustfmt::skip]
						Self::emit_list(f, &[
							I::I32Or, I::If(BlockType::Empty),
								I::LocalGet(0), I::LocalGet(1), I::I32Const(operator as i32),
						]);
						s.call(f, super::super::super::time::time_arithmetic::TIME_ARITHMETIC);
						Self::emit_list(f, &[I::Return, I::End]);
					}
					// node_add of two lists (ø is the empty list): their concatenation, `out = out + row`
					if name == NODE_ADD {
						// an Error operand is the sum, as text_concat gives it: errors propagate, and are never joined into a
						// text as their message (`out += tag(x)` of an error, card error-value-kind)
						for side in 0..2 {
							s.emit_field(f, side, 0);
							Self::emit_kind_test(f, Kind::Error as i64, I::I64Eq);
							Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(side), I::Return, I::End]);
						}
						let is_list = |f: &mut Function, operand: u32| {
							s.emit_field(f, operand, 0);
							#[rustfmt::skip]
							Self::emit_list(f, &[
								I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(Kind::List as i64),
								I::I64Eq, I::LocalGet(kind), I::I64Const(Kind::Block as i64), I::I64Eq, I::I32Or,
								I::LocalGet(kind), I::I64Const(Kind::Empty as i64), I::I64Eq, I::I32Or,
							]);
						};
						is_list(f, 0);
						is_list(f, 1);
						#[rustfmt::skip]
						Self::emit_list(f, &[
							I::I32And, I::If(BlockType::Empty),
								I::LocalGet(0), I::LocalGet(1),
						]);
						s.call(f, "list_concat");
						Self::emit_list(f, &[I::Return, I::End]);
						// two texts (or characters): their concatenation, `await a + await b` of two text tasks
						let is_text = |f: &mut Function, operand: u32| {
							s.emit_field(f, operand, 0);
							#[rustfmt::skip]
							Self::emit_list(f, &[
								I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(Kind::Text as i64),
								I::I64Eq, I::LocalGet(kind), I::I64Const(Kind::Codepoint as i64), I::I64Eq, I::I32Or,
							]);
						};
						is_text(f, 0);
						is_text(f, 1);
						Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(1)]);
						s.call(f, super::super::super::text::text_builtins::TEXT_CONCAT);
						Self::emit_list(f, &[I::Return, I::End]);
						// a text and a number: the number's text joined to it, as `1 + "x"` is "1x" when the kinds are known
						is_text(f, 0);
						is_text(f, 1);
						Self::emit_list(f, &[I::I32Or, I::If(BlockType::Empty)]);
						let node_type = s.type_manager.node_type;
						let empty = s.memory_string("");
						for operand in [0, 1] {
							s.emit_structure_as_literal(
								f,
								operand,
								&super::super::super::text::text_builtins::STRUCTURE_KINDS,
								empty,
							);
						}
						Self::emit_list(f, &[I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), I::LocalGet(0)]);
						s.call(f, super::super::super::text::text_builtins::TEXT_OF);
						Self::emit_list(f, &[I::I64Const(crate::type_kinds::SQUARE_LIST_KIND), I::LocalGet(1)]);
						s.call(f, super::super::super::text::text_builtins::TEXT_OF);
						Self::emit_list(
							f,
							&[
								I::RefNull(HeapType::Concrete(node_type)),
								I::StructNew(node_type),
								I::StructNew(node_type),
							],
						);
						let (pointer, length) = s.memory_string("");
						Self::emit_list(f, &[I::I32Const(pointer as i32), I::I32Const(length as i32)]);
						s.call(f, "new_text");
						s.call(f, super::super::super::reflection::library_ops::LIST_JOIN);
						Self::emit_list(f, &[I::Return, I::End]);
					}
					// an Error value fails with its own message (`x!` of ø is "unwrapped ø"), never as "not a number"
					for operand in [0, 1] {
						s.emit_fail_if_error(f, operand);
					}
					// a remainder of a value with uncertainty is not a number here
					let uncertain = s.should_emit_function(super::super::super::numbers::uncertain::UNCERTAIN_NEW)
						&& index < super::super::super::numbers::uncertain::UNCERTAIN_ARITHMETIC.len();
					// a text, a character or a list is no number here: "x" * 2 is no 240
					for operand in [0, 1] {
						s.emit_field(f, operand, 0);
						#[rustfmt::skip]
						Self::emit_list(f, &[
							I::I64Const(KIND_MASK), I::I64And, I::LocalTee(kind), I::I64Const(int_kind), I::I64Ne,
							I::LocalGet(kind), I::I64Const(float_kind), I::I64Ne, I::I32And,
						]);
						if uncertain {
							Self::emit_list(
								f,
								&[I::LocalGet(kind), I::I64Const(Kind::Uncertain as i64), I::I64Ne, I::I32And],
							);
						}
						s.emit_fail_if(f, NOT_A_NUMBER);
					}
					// a value with uncertainty (uncertain.rs) propagates it
					if uncertain {
						s.is_uncertain(f, 0);
						s.is_uncertain(f, 1);
						Self::emit_list(f, &[I::I32Or, I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(1)]);
						s.call(f, super::super::super::numbers::uncertain::UNCERTAIN_ARITHMETIC[index]);
						Self::emit_list(f, &[I::Return, I::End]);
					}
					for operand in [0, 1] {
						s.emit_field(f, operand, 0);
						Self::emit_kind_test(f, float_kind, I::I64Eq);
					}
					f.instruction(&I::I32Or);
					// an exact power of a ratio exponent is no exact number: 2^(7/12) is computed as f64
					if op == Op::Pow {
						Self::emit_list(
							f,
							&[I::If(BlockType::Result(ValType::I32)), I::I32Const(1), I::Else, I::LocalGet(1)],
						);
						s.call(f, "get_int_value");
						s.call(f, "is_ratio");
						f.instruction(&I::End);
					}
					f.instruction(&I::If(BlockType::Result(node_ref)));
					s.emit_node_float_arithmetic(f, op, power);
					s.call(f, "new_float");
					Self::emit_list(f, &[I::Else, I::LocalGet(0)]);
					s.call(f, "get_int_value");
					f.instruction(&I::LocalGet(1));
					s.call(f, "get_int_value");
					// P66: `a % 0` is the catchable error divide_by_zero, as exact_div makes it for `/`
					if matches!(op, Op::Mod | Op::Rem) {
						Self::emit_list(f, &[I::LocalTee(kind), I::I64Eqz]);
						s.emit_fail_if(f, DIVIDE_BY_ZERO);
						f.instruction(&I::LocalGet(kind));
					}
					s.call(f, exact_op);
					s.call(f, "new_int");
					f.instruction(&I::End);
				},
			);
		}
	}

	pub(in crate::wasm_emitter) fn emit_text_as_float(&mut self) {
		if !self.should_emit_function(TEXT_AS_FLOAT) {
			return;
		}
		let node_ref = Ref(self.node_ref(false));
		let float_box = self.type_manager.f64_box_type;
		let mut locals = vec![ValType::I32; 4];
		locals.push(ValType::F64);
		locals.extend([ValType::I32; 4]);
		locals.push(ValType::F64);
		self.runtime_function(TEXT_AS_FLOAT, vec![node_ref], vec![ValType::F64], locals, |s, f| {
			let (pointer, end, negative, digit, value, scale_digits, digits, exponent, exponent_negative, power) =
				(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
			let at_end = [I::LocalGet(pointer), I::LocalGet(end), I::I32GeU];
			let byte = [I::LocalGet(pointer), I::I32Load8U(BYTE)];
			let advance = [I::LocalGet(pointer), I32Const(1), I::I32Add, I::LocalSet(pointer)];
			let kind = |f: &mut Function| {
				s.emit_field(f, 0, 0);
				Self::emit_list(f, &[I::I64Const(KIND_MASK), I::I64And]);
			};
			// a number node converts as it is
			kind(f);
			Self::emit_list(f, &[I::I64Const(Kind::Float as i64), I::I64Eq, I::If(BlockType::Empty)]);
			s.emit_field(f, 0, 1);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::RefCastNonNull(HeapType::Concrete(float_box)),
				I::StructGet { struct_type_index: float_box, field_index: 0 }, I::Return,
				I::End,
			]);
			// an exact number by its value (a ratio too), a digit character by its digit (text_as_int)
			kind(f);
			Self::emit_list(f, &[I::I64Const(Kind::Text as i64), I::I64Ne, I::If(BlockType::Empty), I::LocalGet(0)]);
			s.call(f, TEXT_AS_INT);
			if s.int_runtime() {
				s.call(f, "exact_to_f64")
			} else {
				f.instruction(&I::F64ConvertI64S);
			}
			Self::emit_list(f, &[I::Return, I::End]);
			s.emit_text_bounds(f, pointer, end);
			let sign = |f: &mut Function, negative: u32| Self::emit_skip_sign(f, pointer, digit, negative);
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			sign(f, negative);
			f.instruction(&I::End);
			// digits, with a point among them: value collects them all, scale_digits counts those after the point
			let digits_loop = |f: &mut Function, after_point: bool| {
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, &at_end);
				f.instruction(&I::BrIf(1));
				Self::emit_list(f, &byte);
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I32Const('0' as i32), I::I32Sub, I::LocalTee(digit), I32Const(9), I::I32GtU, I::BrIf(1),
					I::LocalGet(value), I::F64Const(10.0.into()), I::F64Mul, I::LocalGet(digit), I::F64ConvertI32U,
					I::F64Add, I::LocalSet(value),
					I::LocalGet(digits), I32Const(1), I::I32Add, I::LocalSet(digits),
				]);
				if after_point {
					Self::emit_list(f, &[I::LocalGet(scale_digits), I32Const(1), I::I32Add, I::LocalSet(scale_digits)]);
				}
				Self::emit_list(f, &advance);
				Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			};
			digits_loop(f, false);
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			Self::emit_list(f, &byte);
			Self::emit_list(f, &[I32Const('.' as i32), I::I32Eq, I::If(BlockType::Empty)]);
			Self::emit_list(f, &advance);
			digits_loop(f, true);
			Self::emit_list(f, &[I::End, I::End, I::LocalGet(digits), I::I32Eqz]);
			s.emit_fail_if(f, "invalid_number");
			// an exponent: e or E, a sign, digits
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty)]);
			Self::emit_list(f, &byte);
			Self::emit_list(
				f,
				&[I32Const(ASCII_LOWERCASE_BIT), I::I32Or, I32Const('e' as i32), I::I32Eq, I::If(BlockType::Empty)],
			);
			Self::emit_list(f, &advance);
			Self::emit_list(f, &at_end);
			s.emit_fail_if(f, "invalid_number");
			sign(f, exponent_negative);
			Self::emit_list(
				f,
				&[I32Const(0), I::LocalSet(digits), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)],
			);
			Self::emit_list(f, &at_end);
			f.instruction(&I::BrIf(1));
			Self::emit_list(f, &byte);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I32Const('0' as i32), I::I32Sub, I::LocalTee(digit), I32Const(9), I::I32GtU, I::BrIf(1),
				I::LocalGet(exponent), I32Const(10), I::I32Mul, I::LocalGet(digit), I::I32Add, I::LocalSet(exponent),
				I::LocalGet(digits), I32Const(1), I::I32Add, I::LocalSet(digits),
			]);
			Self::emit_list(f, &advance);
			Self::emit_list(f, &[I::Br(0), I::End, I::End, I::LocalGet(digits), I::I32Eqz]);
			s.emit_fail_if(f, "invalid_number");
			Self::emit_list(f, &[I::End, I::End]);
			// nothing may follow
			Self::emit_list(f, &at_end);
			Self::emit_list(f, &[I::I32Eqz]);
			s.emit_fail_if(f, "invalid_number");
			// the power of ten: the written exponent less the digits after the point. One exact power (10^22 at most is
			// exact in f64) multiplies or divides the digits once, so "3.5e-2" is 35 / 1000 = 0.035 like the literal
			// (ten by ten rounds at each step: 0.034999999999999996); beyond 10^±22 the digits go ten by ten up to it,
			// as a power of 10^310 would be ∞ ("1e-310" is no 0)
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I32Const(0), I::LocalGet(exponent), I::I32Sub, I::LocalGet(exponent), I::LocalGet(exponent_negative),
				I::Select, I::LocalGet(scale_digits), I::I32Sub, I::LocalSet(exponent),
			]);
			let ten_by_ten = [
				(EXACT_POWERS_OF_TEN, I::I32GtS, I::F64Mul, I::I32Sub),
				(-EXACT_POWERS_OF_TEN, I::I32LtS, I::F64Div, I::I32Add),
			];
			for (limit, beyond, step, towards_limit) in ten_by_ten {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(exponent), I32Const(limit), beyond, I::I32Eqz, I::BrIf(1),
					I::LocalGet(value), I::F64Const(10.0.into()), step, I::LocalSet(value),
					I::LocalGet(exponent), I32Const(1), towards_limit, I::LocalSet(exponent),
					I::Br(0), I::End, I::End,
				]);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(exponent), I32Const(0), I::I32LtS, I::LocalSet(exponent_negative),
				I::F64Const(1.0.into()), I::LocalSet(power),
			]);
			for (more, towards_zero) in [(I::I32GtS, I::I32Sub), (I::I32LtS, I::I32Add)] {
				#[rustfmt::skip]
				Self::emit_list(f, &[
					I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
					I::LocalGet(exponent), I32Const(0), more, I::I32Eqz, I::BrIf(1),
					I::LocalGet(power), I::F64Const(10.0.into()), I::F64Mul, I::LocalSet(power),
					I::LocalGet(exponent), I32Const(1), towards_zero, I::LocalSet(exponent),
					I::Br(0), I::End, I::End,
				]);
			}
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::LocalGet(value), I::LocalGet(power), I::F64Div,
				I::LocalGet(value), I::LocalGet(power), I::F64Mul,
				I::LocalGet(exponent_negative), I::Select, I::LocalSet(value),
			]);
			#[rustfmt::skip]
			Self::emit_list(f, &[
				I::F64Const(0.0.into()), I::LocalGet(value), I::F64Sub, I::LocalGet(value), I::LocalGet(negative),
				I::Select,
			]);
		});
	}
}

impl WasmGcEmitter {
	/// Arithmetic without an implicit conversion: `"5"+3` and `[1 2]*2` are type errors, `list + list` concatenates.
	/// Returns true when it emitted the whole expression (a Node, or a trap after a recorded type error).
	pub(in crate::wasm_emitter) fn emit_typed_arithmetic(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &crate::operators::Op,
		right: &Node,
	) -> bool {
		match self.arithmetic_type(left, op, right) {
			Kind::Text if *op == crate::operators::Op::Mul => {
				self.emit_arithmetic_type_error(func, left, op, right, Kind::Text)
			}
			Kind::Text => {
				if !self.emit_unbound_operand(func, left, right) {
					self.emit_text_concat(func, left, right);
				}
				true
			}
			Kind::List => {
				self.emit_node_instructions(func, left);
				self.emit_node_instructions(func, right);
				self.emit_call(func, "list_concat");
				true
			}
			kind => self.emit_arithmetic_type_error(func, left, op, right, kind),
		}
	}

	/// `n times text` and `text * n`: the text n times, joined
	pub(in crate::wasm_emitter) fn emit_text_repeat(&mut self, func: &mut Function, text: &Node, count: &Node) {
		let copies = crate::filled_lists::filled_list(
			count.clone(),
			&Node::List(vec![text.clone()], crate::node::Bracket::Square, crate::node::Separator::Space),
		);
		self.emit_node_instructions(func, &super::super::super::join_call(copies.expect("a one-element list"), ""));
	}

	/// `"ab"*2` repeats; the got-it warning names `2 times "ab"`, and for a text spelling a number how to multiply it
	pub(super) fn educate_text_multiplication(written: &str, text: &Node, count: &Node) {
		// a one-character text parses as a codepoint, but `'5'` would read as a code point (decision #35): keep it a text
		let text_written = match text.drop_meta() {
			Node::Char(character) => format!("\"{character}\""),
			_ => crate::normalize::operand_text(text),
		};
		let count_written = crate::normalize::operand_text(count);
		let spells_a_number = match text.drop_meta() {
			Node::Text(digits) => digits.trim().parse::<f64>().is_ok(),
			Node::Char(digit) => digit.is_ascii_digit(),
			_ => false,
		};
		let reason = if spells_a_number {
			format!("got it: the text repeated; to multiply the number it spells: int({text_written})*{count_written}")
		} else {
			"got it: the text repeated".to_string()
		};
		crate::normalize::set_position_of(count);
		crate::errors::diagnostic::educate_once(
			TEXT_REPEAT_TOPIC,
			written,
			&format!("{count_written} times {text_written}"),
			&reason,
		);
	}

	/// In a numeric context any collection or text operand is a type error
	/// `"clicks " + cont` of an undefined cont: the undefined variable at the name, as in number arithmetic, never a
	/// symbol joined as its name. Returns true when it emitted the error
	pub(super) fn emit_unbound_operand(&mut self, func: &mut Function, left: &Node, right: &Node) -> bool {
		let unbound = [left, right].into_iter().find_map(|operand| match operand.drop_meta() {
			Node::Symbol(name) if self.is_unbound(name) => Some((operand, name.clone())),
			_ => None,
		});
		let Some((operand, name)) = unbound else { return false };
		self.note_position(operand);
		self.emit_undefined_variable(func, &name);
		true
	}

	pub(in crate::wasm_emitter) fn emit_arithmetic_type_error(
		&mut self,
		func: &mut Function,
		left: &Node,
		op: &crate::operators::Op,
		right: &Node,
		kind: Kind,
	) -> bool {
		if !matches!(kind, Kind::Error | Kind::List | Kind::Text) {
			return false;
		}
		if self.emit_unbound_operand(func, left, right) {
			return true;
		}
		let (left_kind, right_kind) = (self.get_type(left), self.get_type(right));
		if crate::analyzer::repeats_text(left_kind, op, right_kind) {
			let (text, count) = if right_kind == Kind::Int { (left, right) } else { (right, left) };
			let fractional = matches!(count.drop_meta(), Node::Number(number) if f64::from(*number).fract() != 0.0);
			if !fractional {
				let written =
					format!("{}*{}", crate::normalize::operand_text(left), crate::normalize::operand_text(right));
				Self::educate_text_multiplication(&written, text, count);
				self.emit_text_repeat(func, text, count);
				return true;
			}
		}
		let is_number = |kind: Kind| matches!(kind, Kind::Int | Kind::Float);
		let repeats_or_scales = *op == crate::operators::Op::Mul
			&& ((left_kind == Kind::List && is_number(right_kind))
				|| (is_number(left_kind) && right_kind == Kind::List));
		let repeats_text = match (left_kind, right_kind) {
			(Kind::Text, kind) if is_number(kind) => Some((left, right)),
			(kind, Kind::Text) if is_number(kind) => Some((right, left)),
			_ => None,
		}
		.filter(|_| *op == crate::operators::Op::Mul);
		let fix = if let Some((text, count)) = repeats_text {
			format!(
				"a text repeats a whole number of times: `{} times {}`",
				crate::normalize::operand_text(count),
				crate::normalize::operand_text(text)
			)
		} else if repeats_or_scales {
			"ambiguous: Python repeats the list, NumPy multiplies each element; write `n times [x]` to repeat, or map to multiply".into()
		} else if left_kind == Kind::Codepoint || right_kind == Kind::Codepoint {
			CHARACTER_IS_NO_NUMBER.into()
		} else if left_kind == Kind::List || right_kind == Kind::List {
			"lists only concatenate with lists (+), element-wise arithmetic needs an explicit map".into()
		} else {
			"no implicit conversion, convert explicitly, e.g. int(\"5\") + 3".into()
		};
		self.emit_type_error(func, format!("type error: {left_kind} {op} {right_kind}: {fix}"));
		true
	}

	pub(in crate::wasm_emitter) fn arithmetic_type(
		&self,
		left: &Node,
		op: &crate::operators::Op,
		right: &Node,
	) -> Kind {
		crate::analyzer::arithmetic_kind_of_operands(self.get_type(left), op, self.get_type(right), right, &self.scope)
	}

	/// Arithmetic on a value whose kind only the run time knows (`x*x` of a parameter held as a Node): maybe a float
	pub(in crate::wasm_emitter) fn computes_at_run_time_kind(&self, value: &Node) -> bool {
		match value.drop_meta() {
			Node::Key(left, op, right) if op.is_arithmetic() => {
				crate::analyzer::node_arithmetic(self.get_type(left), op, self.get_type(right)).is_some()
			}
			Node::List(items, crate::node::Bracket::Round, _) if items.len() == 1 => {
				self.computes_at_run_time_kind(&items[0])
			}
			_ => false,
		}
	}
}
