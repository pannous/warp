//! `x ± r` at run time (card plus-minus, notes/plus_minus.md, decision P217): an interval with its value, a $Node of
//! Kind::Uncertain whose data is the f64 array [value, low, high]. The node_* arithmetic (list_ops.rs) hands an
//! uncertain operand to uncertain_add and its siblings, which take the worst case of the endpoints: `(5 ± 1) + (2 ± 1)`
//! is 7 ± 2 and `x - x` is 0 ± 2 (no correlation). A Gaussian `5 ± 1σ` (card plus-minus-gaussian) has a fourth part,
//! its standard deviation, low and high one deviation from the value; its arithmetic propagates linearly and adds
//! independent deviations in quadrature (gaussian_add and its siblings), so `(5 ± 1σ) + (2 ± 1σ)` is 7 ± 1.4σ

use super::float_text::FLOAT_TEXT;
use super::layout::BYTE;
use super::library_ops::NODE_ORDER;
use super::list_ops::TEXT_AS_FLOAT;
use super::similarity::NUMBERS_SIMILAR;
use super::text_builtins::TEXT_CONCAT;
use super::WasmGcEmitter;
use crate::node::Node;
use crate::operators::Op;
use crate::type_kinds::{Kind, KIND_MASK};
use crate::uncertain::{Extremum, CERTAINLY, INTERVAL_WORDS, POSSIBLY, SIGMA};
use wasm_encoder::*;
use Instruction as I;

/// uncertain_new(value: f64, radius: f64) -> node: value - radius .. value + radius
pub const UNCERTAIN_NEW: &str = "uncertain_new";
/// gaussian_new(value: f64, deviation: f64) -> node: a Gaussian `value ± deviationσ`
pub const GAUSSIAN_NEW: &str = "gaussian_new";
/// `(5 ± 1σ) + (2 ± 1)`: which of the two the result would be is unclear, so it fails
pub(super) const INTERVAL_AND_GAUSSIAN: &str = "an_interval_and_a_gaussian_do_not_mix";
/// gaussian_combine(value, a parts, ∂a, b parts, ∂b) -> node
const GAUSSIAN_COMBINE: &str = "gaussian_combine";
/// node_add and its siblings with a Gaussian operand, in NODE_ARITHMETIC's order
const GAUSSIAN_ARITHMETIC: [&str; 4] = ["gaussian_add", "gaussian_sub", "gaussian_mul", "gaussian_div"];
/// A math word's slope at a Gaussian's value: the difference over this share of its deviation on either side
const SLOPE_STEP: f64 = 1e-3;
/// uncertain_parts(node) -> f64 array: an uncertain value's [value, low, high], any other number as [n, n, n]
const UNCERTAIN_PARTS: &str = "uncertain_parts";
/// uncertain_similar(a, b, tolerance) -> i32: `a ≈ b` when the intervals overlap (or the values are numbers_similar)
pub const UNCERTAIN_SIMILAR: &str = "uncertain_similar";
/// uncertain_text(node) -> text: `7.0 ± 2.0`, for list_join (print, str)
pub const UNCERTAIN_TEXT: &str = "uncertain_text";
/// uncertain_order(a, b, op, mode, message pointer, length) -> i32: `a < b` and the other orders of ± values (P224b),
/// op in ORDERINGS' order, mode one of the CertaintyMode
const UNCERTAIN_ORDER: &str = "uncertain_order";
const ORDERINGS: [Op; 4] = [Op::Lt, Op::Gt, Op::Le, Op::Ge];
/// `y certainly < x` the whole interval, `y possibly < x` some of it; a bare `y < x` the compiler did not know to be
/// ± is certainly, warning once when the intervals overlap
#[derive(Clone, Copy)]
enum CertaintyMode {
	Certainly,
	Possibly,
	Bare,
}
/// `5 ± -1`
pub(super) const NEGATIVE_UNCERTAINTY: &str = "an_uncertainty_must_not_be_negative";
/// The arithmetic of node_add, node_sub, node_mul, node_div with an uncertain operand, in NODE_ARITHMETIC's order
pub const UNCERTAIN_ARITHMETIC: [&str; 4] = ["uncertain_add", "uncertain_sub", "uncertain_mul", "uncertain_div"];
/// uncertain_field(node, field) -> node or null: one of UNCERTAIN_FIELDS as a float
const UNCERTAIN_FIELD: &str = "uncertain_field";
const UNCERTAIN_FUNCTIONS: [&str; 6] = [UNCERTAIN_NEW, GAUSSIAN_NEW, GAUSSIAN_COMBINE, UNCERTAIN_PARTS, UNCERTAIN_FIELD, UNCERTAIN_SIMILAR];
const VALUE: i32 = 0;
const LOW: i32 = 1;
const HIGH: i32 = 2;
const DEVIATION: i32 = 3;
const INTERVAL_PARTS: u32 = 3;
/// A Gaussian's parts: [value, low, high, σ], then per independent Gaussian it depends on its id and contribution
/// (∂/∂that · its σ), so a value met twice is the same contribution twice, not an independent one
const GAUSSIAN_PARTS: u32 = 4;
const CONTRIBUTION_PARTS: u32 = 2;
const RADIUS_FIELD: i32 = 3;
/// The fields of an uncertain value, by their index in the parts (RADIUS_FIELD computed): `x.value`, `x.uncertainty`
const UNCERTAIN_FIELDS: [(i32, &[&str]); 4] = [(VALUE, &["value"]), (LOW, &["low"]), (HIGH, &["high"]), (RADIUS_FIELD, &["uncertainty"])];
const PLUS_MINUS_SEPARATOR: &str = " ± ";
/// More decimals than this, or a value of MAX_SCALED units of its place or more, shows as float_text
const MAX_DECIMALS: i32 = 17;
const MAX_SCALED: f64 = 1e17;
/// Both numbers of at most 17 digits with sign and point, the separator and σ
const TEXT_BYTES: i32 = 64;
/// The scaled ± part shows 2 digits: it rounds to 10 at least and below 100 (0.0999… shows as 0.10)
const ROUNDS_TO_TEN: f64 = 9.5;
const ROUNDS_TO_HUNDRED: f64 = 99.5;

/// A program that makes uncertain values gets the whole family: any arithmetic may meet one
pub fn add_dependencies(required: &mut std::collections::HashSet<&'static str>) {
	if required.contains(UNCERTAIN_NEW) || required.contains(GAUSSIAN_NEW) {
		required.extend(UNCERTAIN_FUNCTIONS);
		required.extend(UNCERTAIN_ARITHMETIC);
		required.extend(GAUSSIAN_ARITHMETIC);
		required.extend([TEXT_AS_FLOAT, NUMBERS_SIMILAR, NEGATIVE_UNCERTAINTY, INTERVAL_AND_GAUSSIAN, "new_float", UNCERTAIN_ORDER, NODE_ORDER]);
	}
}

impl WasmGcEmitter {
	/// After text_as_float, before the node arithmetic, which calls these; uncertain_similar comes with the similarity ops
	pub(super) fn emit_uncertain_runtime(&mut self) {
		if !self.should_emit_function(UNCERTAIN_NEW) {
			return;
		}
		self.emit_uncertain_new();
		self.emit_gaussian_new();
		self.emit_gaussian_combine();
		self.emit_uncertain_parts();
		self.emit_uncertain_field();
		let operations = [I::F64Add, I::F64Sub, I::F64Mul, I::F64Div];
		for (name, operation) in GAUSSIAN_ARITHMETIC.into_iter().zip(operations.clone()) {
			self.emit_gaussian_arithmetic(name, operation);
		}
		for ((name, gaussian), operation) in UNCERTAIN_ARITHMETIC.into_iter().zip(GAUSSIAN_ARITHMETIC).zip(operations) {
			self.emit_uncertain_arithmetic(name, gaussian, operation);
		}
		for (word, name, extrema) in &INTERVAL_WORDS {
			if self.should_emit_function(name) {
				self.emit_interval_word_function(word, name, extrema);
			}
		}
	}

	/// `sin(x)`, `√x` of a value held as a Node in a program with ± values: through the word's uncertain_<word>, which maps
	/// an interval. False otherwise, and the caller computes the word on a float
	pub(super) fn emit_interval_word(&mut self, func: &mut Function, word: &str, argument: &Node) -> bool {
		let Some((_, name, _)) = INTERVAL_WORDS.iter().find(|(known, _, _)| *known == word) else { return false };
		if !self.should_emit_function(UNCERTAIN_NEW) || !crate::analyzer::is_run_time_kind(&self.get_type(argument)) {
			return false;
		}
		self.emit_node_instructions(func, argument);
		self.emit_call(func, name);
		true
	}

	/// The f64 on the stack → the word of it
	fn apply_math_word(&mut self, func: &mut Function, word: &str) {
		match word {
			"sqrt" => { func.instruction(&I::F64Sqrt); }
			"abs" => { func.instruction(&I::F64Abs); }
			"cbrt" => { self.emit_libm_call(func, super::LIBM_CBRT); }
			// imported when the program calls the word; a module emitting every runtime function has no call, nor import
			import => { func.instruction(&self.ffi_func_index(import).map_or(I::Unreachable, I::Call)); }
		}
	}

	/// uncertain_<word>(node) -> node: the word of a number, of an interval the least and greatest of it at the ends, or
	/// the extremum the interval reaches
	fn emit_interval_word_function(&mut self, word: &str, name: &'static str, extrema: &[Extremum]) {
		let node_ref = ValType::Ref(self.node_ref(false));
		let (node, parts, low, high) = (0, 1, 2, 3);
		self.runtime_function(name, vec![node_ref], vec![node_ref], vec![self.parts_ref(), ValType::F64, ValType::F64], |s, f| {
			s.is_uncertain(f, node);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(node)]);
			s.call(f, TEXT_AS_FLOAT);
			s.apply_math_word(f, word);
			s.call(f, "new_float");
			Self::emit_list(f, &[I::Return, I::End, I::LocalGet(node)]);
			s.call(f, UNCERTAIN_PARTS);
			f.instruction(&I::LocalSet(parts));
			s.is_gaussian(f, parts);
			f.instruction(&I::If(BlockType::Empty));
			s.gaussian_word(f, word, parts, (low, high));
			Self::emit_list(f, &[I::Return, I::End]);
			for (end, local) in [(LOW, low), (HIGH, high)] {
				Self::emit_list(f, &s.part(parts, end));
				s.apply_math_word(f, word);
				f.instruction(&I::LocalSet(local));
			}
			Self::emit_list(f, &[
				I::LocalGet(low), I::LocalGet(high), I::F64Min, I::LocalGet(low), I::LocalGet(high), I::F64Max,
				I::LocalSet(high), I::LocalSet(low),
			]);
			for extremum in extrema {
				let bound = if extremum.high { high } else { low };
				Self::emit_list(f, &[I::F64Const(extremum.value.into()), I::LocalGet(bound)]);
				s.reaches(f, parts, extremum);
				Self::emit_list(f, &[I::Select, I::LocalSet(bound)]);
			}
			f.instruction(&I::I64Const(Kind::Uncertain as i64));
			Self::emit_list(f, &s.part(parts, VALUE));
			s.apply_math_word(f, word);
			Self::emit_list(f, &[I::LocalGet(low), I::LocalGet(high)]);
			s.new_uncertain_node(f);
		});
	}

	/// The Gaussian of the word of the Gaussian in `parts`: the word of its value, its contributions times the word's
	/// slope there (the difference across SLOPE_STEP of its σ on either side, so a word needs no formula for its slope).
	/// Uses the f64 locals `at` and `step`; leaves the result node
	fn gaussian_word(&mut self, func: &mut Function, word: &str, parts: u32, (at, step): (u32, u32)) {
		Self::emit_list(func, &self.part(parts, VALUE));
		func.instruction(&I::LocalSet(at));
		Self::emit_list(func, &self.part(parts, DEVIATION));
		Self::emit_list(func, &[I::F64Const(SLOPE_STEP.into()), I::F64Mul, I::LocalSet(step), I::LocalGet(at)]);
		self.apply_math_word(func, word);
		func.instruction(&I::LocalGet(parts));
		for sign in [I::F64Add, I::F64Sub] {
			Self::emit_list(func, &[I::LocalGet(at), I::LocalGet(step), sign]);
			self.apply_math_word(func, word);
		}
		// (f(x + h) - f(x - h)) / 2h, 0 without a σ
		Self::emit_list(func, &[I::F64Sub, I::LocalGet(step), I::F64Const(2.0.into()), I::F64Mul, I::F64Div]);
		Self::emit_list(func, &[I::F64Const(0.0.into()), I::LocalGet(step), I::F64Const(0.0.into()), I::F64Gt, I::Select]);
		Self::emit_list(func, &[I::LocalGet(parts), I::F64Const(0.0.into())]);
		self.call(func, GAUSSIAN_COMBINE);
	}

	/// Push an i32: does the interval in `parts` reach the extremum: `at`, or the first `at + k·every` from its low end
	fn reaches(&self, func: &mut Function, parts: u32, extremum: &Extremum) {
		let at = I::F64Const(extremum.at.into());
		if extremum.every == 0.0 {
			func.instruction(&at);
			Self::emit_list(func, &self.part(parts, LOW));
			Self::emit_list(func, &[I::F64Ge, at]);
			Self::emit_list(func, &self.part(parts, HIGH));
			Self::emit_list(func, &[I::F64Le, I::I32And]);
			return;
		}
		let every = I::F64Const(extremum.every.into());
		Self::emit_list(func, &self.part(parts, LOW));
		Self::emit_list(func, &[at.clone(), I::F64Sub, every.clone(), I::F64Div, I::F64Ceil, every, I::F64Mul, at, I::F64Add]);
		Self::emit_list(func, &self.part(parts, HIGH));
		func.instruction(&I::F64Le);
	}

	/// Push an i32: is the Node in `local` uncertain
	pub(super) fn is_uncertain(&self, func: &mut Function, local: u32) {
		self.emit_field(func, local, 0);
		Self::emit_list(func, &[I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Uncertain as i64), I::I64Eq]);
	}

	fn parts_array(&self) -> u32 {
		self.type_manager.float_array_type
	}

	fn parts_ref(&self) -> ValType {
		ValType::Ref(RefType { nullable: false, heap_type: HeapType::Concrete(self.parts_array()) })
	}

	/// [I::LocalGet(parts), index, ArrayGet]: one part of the array in `parts`
	fn part(&self, parts: u32, index: i32) -> [I<'static>; 3] {
		[I::LocalGet(parts), I::I32Const(index), I::ArrayGet(self.parts_array())]
	}

	/// Push the ± part of the parts in `parts`: how far the interval reaches from the value, on its farther side
	fn push_radius(&self, func: &mut Function, parts: u32) {
		Self::emit_list(func, &self.part(parts, HIGH));
		Self::emit_list(func, &self.part(parts, VALUE));
		func.instruction(&I::F64Sub);
		Self::emit_list(func, &self.part(parts, VALUE));
		Self::emit_list(func, &self.part(parts, LOW));
		Self::emit_list(func, &[I::F64Sub, I::F64Max]);
	}

	/// Push an i32: are the parts in `parts` a Gaussian's
	fn is_gaussian(&self, func: &mut Function, parts: u32) {
		Self::emit_list(func, &[I::LocalGet(parts), I::ArrayLen, I::I32Const(INTERVAL_PARTS as i32), I::I32GtU]);
	}

	/// The uncertain node of the kind and the three f64 on the stack
	fn new_uncertain_node(&self, func: &mut Function) {
		self.new_uncertain_node_of(func, INTERVAL_PARTS);
	}

	fn new_uncertain_node_of(&self, func: &mut Function, parts: u32) {
		let node = self.type_manager.node_type;
		Self::emit_list(func, &[
			I::ArrayNewFixed { array_type_index: self.parts_array(), array_size: parts },
			I::RefNull(HeapType::Concrete(node)), I::StructNew(node),
		]);
	}

	fn emit_uncertain_new(&mut self) {
		let (value, radius) = (0, 1);
		let node_ref = ValType::Ref(self.node_ref(false));
		self.runtime_function(UNCERTAIN_NEW, vec![ValType::F64; 2], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::LocalGet(radius), I::F64Const(0.0.into()), I::F64Lt]);
			s.emit_fail_if(f, NEGATIVE_UNCERTAINTY);
			Self::emit_list(f, &[
				I::I64Const(Kind::Uncertain as i64), I::LocalGet(value),
				I::LocalGet(value), I::LocalGet(radius), I::F64Sub,
				I::LocalGet(value), I::LocalGet(radius), I::F64Add,
			]);
			s.new_uncertain_node(f);
		});
	}

	/// gaussian_new(value, σ): a Gaussian of its own, the one contribution σ under a new id
	fn emit_gaussian_new(&mut self) {
		self.globals.global(GlobalType { val_type: ValType::F64, mutable: true, shared: false }, &ConstExpr::f64_const(0.0.into()));
		self.next_global_idx += 1;
		let last_id = self.next_global_idx - 1;
		let (value, deviation) = (0, 1);
		let node_ref = ValType::Ref(self.node_ref(false));
		self.runtime_function(GAUSSIAN_NEW, vec![ValType::F64; 2], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::LocalGet(deviation), I::F64Const(0.0.into()), I::F64Lt]);
			s.emit_fail_if(f, NEGATIVE_UNCERTAINTY);
			Self::emit_list(f, &[
				I::I64Const(Kind::Uncertain as i64), I::LocalGet(value),
				I::LocalGet(value), I::LocalGet(deviation), I::F64Sub,
				I::LocalGet(value), I::LocalGet(deviation), I::F64Add, I::LocalGet(deviation),
				I::GlobalGet(last_id), I::F64Const(1.0.into()), I::F64Add, I::GlobalSet(last_id), I::GlobalGet(last_id), I::LocalGet(deviation),
			]);
			s.new_uncertain_node_of(f, GAUSSIAN_PARTS + CONTRIBUTION_PARTS);
		});
	}

	/// Push the i32 number of contributions in the parts in `parts`: none for a number's [n, n, n]
	fn contributions(parts: u32) -> [I<'static>; 12] {
		[
			I::LocalGet(parts), I::ArrayLen, I::I32Const(GAUSSIAN_PARTS as i32), I::I32Sub, I::I32Const(1), I::I32ShrS, I::I32Const(0),
			I::LocalGet(parts), I::ArrayLen, I::I32Const(INTERVAL_PARTS as i32), I::I32GtU, I::Select,
		]
	}

	/// The index of the id (`offset` 0) or the amount (1) of the contribution numbered by the i32 local `entry`
	fn contribution(entry: u32, offset: i32) -> [I<'static>; 5] {
		[I::LocalGet(entry), I::I32Const(1), I::I32Shl, I::I32Const(GAUSSIAN_PARTS as i32 + offset), I::I32Add]
	}

	/// `for counter in 0..limit { body }`, limit an i32 the instructions push
	fn repeat(func: &mut Function, counter: u32, limit: &[I<'static>], body: impl FnOnce(&mut Function)) {
		Self::emit_list(func, &[I::I32Const(0), I::LocalSet(counter), I::Block(BlockType::Empty), I::Loop(BlockType::Empty), I::LocalGet(counter)]);
		Self::emit_list(func, limit);
		Self::emit_list(func, &[I::I32GeS, I::BrIf(1)]);
		body(func);
		Self::emit_list(func, &[I::LocalGet(counter), I::I32Const(1), I::I32Add, I::LocalSet(counter), I::Br(0), I::End, I::End]);
	}

	/// gaussian_combine(value, a parts, ∂a, b parts, ∂b) -> node: the Gaussian `value` whose contributions are a's times
	/// ∂a and b's times ∂b, those of one id summed: `x - x` cancels, `x * x` doubles (Measurements.jl's linear
	/// propagation). Its σ is √(Σ contribution²)
	fn emit_gaussian_combine(&mut self) {
		let array = self.parts_array();
		let node = self.type_manager.node_type;
		let node_ref = ValType::Ref(self.node_ref(false));
		let params = vec![ValType::F64, self.parts_ref(), ValType::F64, self.parts_ref(), ValType::F64];
		let locals = vec![self.parts_ref(), ValType::I32, ValType::I32, ValType::I32, ValType::F64, ValType::F64, self.parts_ref()];
		self.runtime_function(GAUSSIAN_COMBINE, params, vec![node_ref], locals, |_, f| {
			let (value, a, a_slope, b, b_slope) = (0, 1, 2, 3, 4);
			let (sums, count, index, search, id, square_sum, parts) = (5, 6, 7, 8, 9, 10, 11);
			let get = |parts: u32, entry: u32, offset: i32| {
				let mut instructions = vec![I::LocalGet(parts)];
				instructions.extend(Self::contribution(entry, offset));
				instructions.push(I::ArrayGet(array));
				instructions
			};
			let set_in_sums = |f: &mut Function, entry: u32, offset: i32, amount: &[I<'static>]| {
				f.instruction(&I::LocalGet(sums));
				Self::emit_list(f, &Self::contribution(entry, offset));
				Self::emit_list(f, amount);
				f.instruction(&I::ArraySet(array));
			};
			let scaled = |parts: u32, slope: u32| {
				let mut amount = get(parts, index, 1);
				amount.extend([I::LocalGet(slope), I::F64Mul]);
				amount
			};
			let next = |f: &mut Function| Self::emit_list(f, &[I::LocalGet(count), I::I32Const(1), I::I32Add, I::LocalSet(count)]);

			f.instruction(&I::I32Const(GAUSSIAN_PARTS as i32));
			Self::emit_list(f, &Self::contributions(a));
			Self::emit_list(f, &Self::contributions(b));
			Self::emit_list(f, &[I::I32Add, I::I32Const(1), I::I32Shl, I::I32Add, I::ArrayNewDefault(array), I::LocalSet(sums)]);
			Self::repeat(f, index, &Self::contributions(a), |f| {
				set_in_sums(f, count, 0, &get(a, index, 0));
				set_in_sums(f, count, 1, &scaled(a, a_slope));
				next(f);
			});
			Self::repeat(f, index, &Self::contributions(b), |f| {
				Self::emit_list(f, &get(b, index, 0));
				Self::emit_list(f, &[I::LocalSet(id), I::I32Const(0), I::LocalSet(search)]);
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, &[I::LocalGet(search), I::LocalGet(count), I::I32GeS, I::BrIf(1)]);
				Self::emit_list(f, &get(sums, search, 0));
				Self::emit_list(f, &[I::LocalGet(id), I::F64Eq, I::If(BlockType::Empty)]);
				let mut summed = get(sums, search, 1);
				summed.extend(scaled(b, b_slope));
				summed.push(I::F64Add);
				set_in_sums(f, search, 1, &summed);
				Self::emit_list(f, &[I::Br(3), I::End]);
				Self::emit_list(f, &[I::LocalGet(search), I::I32Const(1), I::I32Add, I::LocalSet(search), I::Br(0), I::End, I::End]);
				set_in_sums(f, count, 0, &[I::LocalGet(id)]);
				set_in_sums(f, count, 1, &scaled(b, b_slope));
				next(f);
				f.instruction(&I::End);
			});
			Self::emit_list(f, &[I::F64Const(0.0.into()), I::LocalSet(square_sum)]);
			Self::repeat(f, index, &[I::LocalGet(count)], |f| {
				Self::emit_list(f, &get(sums, index, 1));
				f.instruction(&I::LocalTee(id));
				Self::emit_list(f, &[I::LocalGet(id), I::F64Mul, I::LocalGet(square_sum), I::F64Add, I::LocalSet(square_sum)]);
			});
			Self::emit_list(f, &[
				I::I32Const(GAUSSIAN_PARTS as i32), I::LocalGet(count), I::I32Const(1), I::I32Shl, I::I32Add, I::ArrayNewDefault(array), I::LocalSet(parts),
				I::LocalGet(parts), I::I32Const(GAUSSIAN_PARTS as i32), I::LocalGet(sums), I::I32Const(GAUSSIAN_PARTS as i32),
				I::LocalGet(count), I::I32Const(1), I::I32Shl, I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
				I::LocalGet(square_sum), I::F64Sqrt, I::LocalSet(square_sum),
			]);
			let parts_of = [
				(VALUE, vec![I::LocalGet(value)]),
				(LOW, vec![I::LocalGet(value), I::LocalGet(square_sum), I::F64Sub]),
				(HIGH, vec![I::LocalGet(value), I::LocalGet(square_sum), I::F64Add]),
				(DEVIATION, vec![I::LocalGet(square_sum)]),
			];
			for (index, amount) in parts_of {
				Self::emit_list(f, &[I::LocalGet(parts), I::I32Const(index)]);
				Self::emit_list(f, &amount);
				f.instruction(&I::ArraySet(array));
			}
			Self::emit_list(f, &[I::I64Const(Kind::Uncertain as i64), I::LocalGet(parts), I::RefNull(HeapType::Concrete(node)), I::StructNew(node)]);
		});
	}

	/// gaussian_<op>(a, b) -> node: the operation on the values, the contributions through its partial derivatives
	/// (gaussian_combine). A number contributes nothing, an interval fails
	fn emit_gaussian_arithmetic(&mut self, name: &'static str, operation: I<'static>) {
		let node_ref = ValType::Ref(self.node_ref(false));
		let (a, b, a_value, b_value) = (2, 3, 4, 5);
		let locals = vec![self.parts_ref(), self.parts_ref(), ValType::F64, ValType::F64];
		self.runtime_function(name, vec![node_ref, node_ref], vec![node_ref], locals, |s, f| {
			for (node, parts, value) in [(0, a, a_value), (1, b, b_value)] {
				f.instruction(&I::LocalGet(node));
				s.call(f, UNCERTAIN_PARTS);
				f.instruction(&I::LocalTee(parts));
				Self::emit_list(f, &[I::ArrayLen, I::I32Const(INTERVAL_PARTS as i32), I::I32Eq]);
				s.is_uncertain(f, node);
				f.instruction(&I::I32And);
				s.emit_fail_if(f, INTERVAL_AND_GAUSSIAN);
				Self::emit_list(f, &s.part(parts, VALUE));
				f.instruction(&I::LocalSet(value));
			}
			let one = I::F64Const(1.0.into());
			let (a_slope, b_slope): (Vec<I<'static>>, Vec<I<'static>>) = match operation {
				I::F64Sub => (vec![one.clone()], vec![I::F64Const((-1.0).into())]),
				I::F64Mul => (vec![I::LocalGet(b_value)], vec![I::LocalGet(a_value)]),
				I::F64Div => (
					vec![one, I::LocalGet(b_value), I::F64Div],
					vec![I::LocalGet(a_value), I::F64Neg, I::LocalGet(b_value), I::LocalGet(b_value), I::F64Mul, I::F64Div],
				),
				_ => (vec![one.clone()], vec![one]),
			};
			Self::emit_list(f, &[I::LocalGet(a_value), I::LocalGet(b_value), operation, I::LocalGet(a)]);
			Self::emit_list(f, &a_slope);
			f.instruction(&I::LocalGet(b));
			Self::emit_list(f, &b_slope);
			s.call(f, GAUSSIAN_COMBINE);
		});
	}

	fn emit_uncertain_parts(&mut self) {
		let array = self.parts_array();
		let node_ref = ValType::Ref(self.node_ref(false));
		let parts_ref = self.parts_ref();
		self.runtime_function(UNCERTAIN_PARTS, vec![node_ref], vec![parts_ref], vec![ValType::F64], |s, f| {
			s.is_uncertain(f, 0);
			Self::emit_list(f, &[I::If(BlockType::Empty)]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(array)), I::Return, I::End, I::LocalGet(0)]);
			s.call(f, TEXT_AS_FLOAT);
			Self::emit_list(f, &[I::LocalTee(1), I::LocalGet(1), I::LocalGet(1), I::ArrayNewFixed { array_type_index: array, array_size: 3 }]);
		});
	}

	/// uncertain_<op>(a, b) -> node: the operation on the values, the bounds the least and greatest of it on the four
	/// pairs of endpoints (for + and - that is low with low and high with high, or crosswise)
	fn emit_uncertain_arithmetic(&mut self, name: &'static str, gaussian: &'static str, operation: I<'static>) {
		let node_ref = ValType::Ref(self.node_ref(false));
		let (a, b, low, high) = (2, 3, 4, 5);
		let locals = vec![self.parts_ref(), self.parts_ref(), ValType::F64, ValType::F64];
		self.runtime_function(name, vec![node_ref, node_ref], vec![node_ref], locals, |s, f| {
			for (node, parts) in [(0, a), (1, b)] {
				f.instruction(&I::LocalGet(node));
				s.call(f, UNCERTAIN_PARTS);
				f.instruction(&I::LocalSet(parts));
			}
			s.is_gaussian(f, a);
			s.is_gaussian(f, b);
			Self::emit_list(f, &[I::I32Or, I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(1)]);
			s.call(f, gaussian);
			Self::emit_list(f, &[I::Return, I::End]);
			let pairs = [(LOW, LOW), (LOW, HIGH), (HIGH, LOW), (HIGH, HIGH)];
			for (bound, fold) in [(low, I::F64Min), (high, I::F64Max)] {
				for (index, (a_end, b_end)) in pairs.into_iter().enumerate() {
					Self::emit_list(f, &s.part(a, a_end));
					Self::emit_list(f, &s.part(b, b_end));
					f.instruction(&operation);
					if index > 0 {
						f.instruction(&fold);
					}
				}
				f.instruction(&I::LocalSet(bound));
			}
			f.instruction(&I::I64Const(Kind::Uncertain as i64));
			Self::emit_list(f, &s.part(a, VALUE));
			Self::emit_list(f, &s.part(b, VALUE));
			Self::emit_list(f, &[operation, I::LocalGet(low), I::LocalGet(high)]);
			s.new_uncertain_node(f);
		});
	}

	/// uncertain_text(node) -> text: `7.0 ± 2.0`, as uncertain::Uncertain shows it read back (P219): the ± part to
	/// 2 significant digits, the value to the same place; a ± part of 0 or ∞, or digits beyond an i64, as float_text
	/// shows each. After float_text and text_concat, before list_join, which calls it
	pub(super) fn emit_uncertain_text(&mut self) {
		if !self.should_emit_function(UNCERTAIN_TEXT) {
			return;
		}
		self.emit_text_heap_global();
		let node_ref = ValType::Ref(self.node_ref(false));
		let separator = self.allocate_string(PLUS_MINUS_SEPARATOR);
		let sigma = self.allocate_string(SIGMA);
		let mut locals = vec![self.parts_ref()];
		locals.extend([ValType::F64; 4]);
		locals.extend([ValType::I32; 3]);
		locals.extend([ValType::I64; 2]);
		self.runtime_function(UNCERTAIN_TEXT, vec![node_ref], vec![node_ref], locals, |s, f| {
			let (parts, value, radius, scale, x, decimals, address, position, digits, power) = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
			let new_text = |f: &mut Function, (text_address, length): (u32, u32)| {
				Self::emit_list(f, &[I::I32Const(text_address as i32), I::I32Const(length as i32)]);
				s.call(f, "new_text");
			};
			let each_as_float_text = |f: &mut Function| {
				f.instruction(&I::LocalGet(value));
				s.call(f, FLOAT_TEXT);
				new_text(f, separator);
				s.call(f, TEXT_CONCAT);
				f.instruction(&I::LocalGet(radius));
				s.call(f, FLOAT_TEXT);
				s.call(f, TEXT_CONCAT);
				s.is_gaussian(f, parts);
				f.instruction(&I::If(BlockType::Result(node_ref)));
				new_text(f, sigma);
				f.instruction(&I::Else);
				new_text(f, (sigma.0, 0));
				f.instruction(&I::End);
				s.call(f, TEXT_CONCAT);
				f.instruction(&I::Return);
			};
			let put = |f: &mut Function, byte: u8| {
				Self::emit_list(f, &[I::LocalGet(position), I::I32Const(byte as i32), I::I32Store8(BYTE)]);
				Self::emit_list(f, &[I::LocalGet(position), I::I32Const(1), I::I32Add, I::LocalSet(position)]);
			};
			let write_decimal = |f: &mut Function, number: &[I<'static>]| {
				f.instruction(&I::LocalGet(position));
				Self::emit_list(f, number);
				s.call(f, "int_to_decimal");
			};
			// the number in `x` to the place of `scale`: rounded half away from zero to whole units of 1/scale, or
			// half to even at `decimals` digits after the point, as Rust's `{:.*}`
			let fixed = |f: &mut Function| {
				Self::emit_list(f, &[I::LocalGet(x), I::F64Const(0.0.into()), I::F64Lt, I::If(BlockType::Empty)]);
				put(f, b'-');
				Self::emit_list(f, &[I::LocalGet(x), I::F64Neg, I::LocalSet(x), I::End]);
				Self::emit_list(f, &[I::LocalGet(decimals), I::I32Eqz, I::If(BlockType::Empty)]);
				write_decimal(f, &[
					I::LocalGet(x), I::LocalGet(scale), I::F64Mul, I::F64Const(0.5.into()), I::F64Add, I::F64Floor, I::I64TruncF64S,
					I::F64Const(1.0.into()), I::LocalGet(scale), I::F64Div, I::F64Nearest, I::I64TruncF64S, I::I64Mul,
				]);
				Self::emit_list(f, &[I::LocalGet(position), I::I32Add, I::LocalSet(position), I::Else]);
				Self::emit_list(f, &[I::LocalGet(scale), I::I64TruncF64S, I::LocalSet(power)]);
				Self::emit_list(f, &[I::LocalGet(x), I::LocalGet(scale), I::F64Mul, I::F64Nearest, I::I64TruncF64S, I::LocalSet(digits)]);
				write_decimal(f, &[I::LocalGet(digits), I::LocalGet(power), I::I64DivS]);
				Self::emit_list(f, &[I::LocalGet(position), I::I32Add, I::LocalSet(position)]);
				// the fraction with its leading zeros: power + fraction, its leading 1 overwritten by the point
				write_decimal(f, &[I::LocalGet(power), I::LocalGet(digits), I::LocalGet(power), I::I64RemS, I::I64Add]);
				f.instruction(&I::Drop);
				put(f, b'.');
				Self::emit_list(f, &[I::LocalGet(position), I::LocalGet(decimals), I::I32Add, I::LocalSet(position), I::End]);
			};
			let while_true = |f: &mut Function, condition: &[I<'static>], body: &[I<'static>]| {
				Self::emit_list(f, &[I::Block(BlockType::Empty), I::Loop(BlockType::Empty)]);
				Self::emit_list(f, condition);
				Self::emit_list(f, &[I::I32Eqz, I::BrIf(1)]);
				Self::emit_list(f, body);
				Self::emit_list(f, &[I::Br(0), I::End, I::End]);
			};

			f.instruction(&I::LocalGet(0));
			s.call(f, UNCERTAIN_PARTS);
			f.instruction(&I::LocalSet(parts));
			Self::emit_list(f, &s.part(parts, VALUE));
			f.instruction(&I::LocalSet(value));
			s.push_radius(f, parts);
			f.instruction(&I::LocalTee(radius));
			Self::emit_list(f, &[I::F64Const(0.0.into()), I::F64Gt, I::LocalGet(radius), I::F64Const(f64::INFINITY.into()), I::F64Lt, I::I32And, I::I32Eqz, I::If(BlockType::Empty)]);
			each_as_float_text(f);
			f.instruction(&I::End);
			// scale: the power of ten that puts the ± part, rounded, in [10, 100): its 2 significant digits before the point
			let scaled_radius = [I::LocalGet(radius), I::LocalGet(scale), I::F64Mul];
			Self::emit_list(f, &[I::F64Const(1.0.into()), I::LocalSet(scale)]);
			let mut too_small = scaled_radius.to_vec();
			too_small.extend([I::F64Const(ROUNDS_TO_TEN.into()), I::F64Lt, I::LocalGet(decimals), I::I32Const(MAX_DECIMALS), I::I32LeS, I::I32And]);
			while_true(f, &too_small, &[
				I::LocalGet(scale), I::F64Const(10.0.into()), I::F64Mul, I::LocalSet(scale),
				I::LocalGet(decimals), I::I32Const(1), I::I32Add, I::LocalSet(decimals),
			]);
			let mut too_large = scaled_radius.to_vec();
			too_large.extend([I::F64Const(ROUNDS_TO_HUNDRED.into()), I::F64Ge]);
			while_true(f, &too_large, &[I::LocalGet(scale), I::F64Const(10.0.into()), I::F64Div, I::LocalSet(scale)]);
			Self::emit_list(f, &[
				I::LocalGet(decimals), I::I32Const(MAX_DECIMALS), I::I32GtS,
				I::LocalGet(value), I::F64Abs, I::LocalGet(scale), I::F64Mul, I::F64Const(MAX_SCALED.into()), I::F64Lt, I::I32Eqz,
				I::I32Or, I::If(BlockType::Empty),
			]);
			each_as_float_text(f);
			f.instruction(&I::End);

			Self::emit_list(f, &[I::I32Const(TEXT_BYTES), I::LocalSet(position)]);
			s.emit_text_allocation(f, position, address);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalSet(position), I::LocalGet(value), I::LocalSet(x)]);
			fixed(f);
			for byte in PLUS_MINUS_SEPARATOR.bytes() {
				put(f, byte);
			}
			Self::emit_list(f, &[I::LocalGet(radius), I::LocalSet(x)]);
			fixed(f);
			s.is_gaussian(f, parts);
			f.instruction(&I::If(BlockType::Empty));
			for byte in SIGMA.bytes() {
				put(f, byte);
			}
			f.instruction(&I::End);
			Self::emit_list(f, &[I::LocalGet(address), I::LocalGet(position), I::LocalGet(address), I::I32Sub]);
			s.call(f, "new_text");
		});
	}

	/// uncertain_field(node, field) -> node or null: `x.value`, `x.low`, `x.high`, `x.uncertainty` of an uncertain x as
	/// a float, null for any other node (then the general lookup decides); `field` is the index in UNCERTAIN_FIELDS
	fn emit_uncertain_field(&mut self) {
		let array = self.parts_array();
		let node_ref = ValType::Ref(self.node_ref(false));
		let nullable = ValType::Ref(self.node_ref(true));
		let (node, field, parts) = (0, 1, 2);
		self.runtime_function(UNCERTAIN_FIELD, vec![node_ref, ValType::I32], vec![nullable], vec![self.parts_ref()], |s, f| {
			s.is_uncertain(f, node);
			Self::emit_list(f, &[I::I32Eqz, I::If(BlockType::Empty), I::RefNull(HeapType::Concrete(s.type_manager.node_type)), I::Return, I::End]);
			s.emit_field(f, node, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(array)), I::LocalSet(parts)]);
			Self::emit_list(f, &[I::LocalGet(field), I::I32Const(RADIUS_FIELD), I::I32Eq, I::If(BlockType::Result(ValType::F64))]);
			s.push_radius(f, parts);
			Self::emit_list(f, &[I::Else, I::LocalGet(parts), I::LocalGet(field), I::ArrayGet(array), I::End]);
			s.call(f, "new_float");
		});
	}

	/// Push `node.name` of an uncertain node, else nothing found: the general lookup of `target.name` follows. The node is
	/// in `held`, inside the lookup's block, which a found field leaves
	pub(super) fn emit_uncertain_field_lookup(&mut self, func: &mut Function, held: u32, name: &str) {
		let Some(field) = UNCERTAIN_FIELDS.iter().position(|(_, names)| names.contains(&name)) else { return };
		if !self.should_emit_function(UNCERTAIN_NEW) {
			return;
		}
		Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull, I::I32Const(UNCERTAIN_FIELDS[field].0)]);
		self.call(func, UNCERTAIN_FIELD);
		func.instruction(&I::BrOnNonNull(0));
	}

	/// Replace the node in `local` by its value when it is uncertain: `<` and the other orders compare values
	pub(super) fn emit_uncertain_as_value(&self, func: &mut Function, local: u32) {
		if !self.should_emit_function(UNCERTAIN_NEW) {
			return;
		}
		let node_ref = ValType::Ref(self.node_ref(false));
		Self::emit_list(func, &[I::Block(BlockType::Result(node_ref)), I::LocalGet(local), I::I32Const(VALUE)]);
		self.call(func, UNCERTAIN_FIELD);
		Self::emit_list(func, &[I::BrOnNonNull(0), I::LocalGet(local), I::End, I::LocalSet(local)]);
	}

	/// After numbers_similar, before values_similar
	pub(super) fn emit_uncertain_similar(&mut self) {
		if !self.should_emit_function(UNCERTAIN_SIMILAR) {
			return;
		}
		let (a, b, tolerance, a_parts, b_parts) = (0, 1, 2, 3, 4);
		let node_ref = ValType::Ref(self.node_ref(false));
		let locals = vec![self.parts_ref(), self.parts_ref()];
		self.runtime_function(UNCERTAIN_SIMILAR, vec![node_ref, node_ref, ValType::F64], vec![ValType::I32], locals, |s, f| {
			for (node, parts) in [(a, a_parts), (b, b_parts)] {
				f.instruction(&I::LocalGet(node));
				s.call(f, UNCERTAIN_PARTS);
				f.instruction(&I::LocalSet(parts));
			}
			for (lower, upper) in [(a_parts, b_parts), (b_parts, a_parts)] {
				Self::emit_list(f, &s.part(lower, LOW));
				Self::emit_list(f, &s.part(upper, HIGH));
				f.instruction(&I::F64Le);
			}
			f.instruction(&I::I32And);
			Self::emit_list(f, &s.part(a_parts, VALUE));
			Self::emit_list(f, &s.part(b_parts, VALUE));
			f.instruction(&I::LocalGet(tolerance));
			s.call(f, NUMBERS_SIMILAR);
			f.instruction(&I::I32Or);
		});
	}
}

impl WasmGcEmitter {
	/// `certainly(y < x)`, `possibly(y < x)` (crate::uncertain::lower_certainty): an i64 0 or 1
	pub(super) fn emit_certainty(&mut self, func: &mut Function, word: &str, comparison: &Node) {
		let Node::Key(left, op, right) = comparison.drop_meta() else {
			return self.emit_type_error(func, format!("{word} compares with <, >, <= or >=, not {}", comparison.serialize()));
		};
		if !op.is_ordering() {
			return self.emit_type_error(func, format!("{word} compares with <, >, <= or >=, not {op}"));
		}
		if !self.should_emit_function(UNCERTAIN_ORDER) {
			return self.emit_int_operands_op(func, left, op, right); // no ± in the program: the comparison itself
		}
		self.emit_node_instructions(func, left);
		self.emit_node_instructions(func, right);
		let mode = if word == CERTAINLY { CertaintyMode::Certainly } else { debug_assert_eq!(word, POSSIBLY); CertaintyMode::Possibly };
		self.emit_uncertain_order_call(func, *op, mode, (0, 0));
	}

	/// After both operands of a bare ordering: the i64 verdict, or false with `overlap_warning` when ± values overlap.
	/// False when the program has no ± values (the caller orders them itself)
	pub(super) fn emit_bare_uncertain_order(&mut self, func: &mut Function, left: &Node, op: Op, right: &Node) -> bool {
		if !self.should_emit_function(UNCERTAIN_ORDER) {
			return false;
		}
		let warning = self.located(crate::uncertain::overlap_warning(left, op, right));
		let message = self.allocate_string(&warning);
		self.emit_uncertain_order_call(func, op, CertaintyMode::Bare, message);
		true
	}

	fn emit_uncertain_order_call(&mut self, func: &mut Function, op: Op, mode: CertaintyMode, (pointer, length): (u32, u32)) {
		let op_index = ORDERINGS.iter().position(|ordering| *ordering == op).expect("an ordering");
		Self::emit_list(func, &[I::I32Const(op_index as i32), I::I32Const(mode as i32), I::I32Const(pointer as i32), I::I32Const(length as i32)]);
		self.emit_call(func, UNCERTAIN_ORDER);
		func.instruction(&I::I64ExtendI32U);
	}

	/// uncertain_order: numbers as intervals of one point, anything else by node_order. After node_order
	pub(super) fn emit_uncertain_order(&mut self) {
		if !self.should_emit_function(UNCERTAIN_ORDER) {
			return;
		}
		let host_warn = self.ctx.func_registry.get(super::text_builtins::HOST_WARN).map(|function| function.call_index as u32);
		let warned = host_warn.map(|_| {
			self.globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(0));
			self.next_global_idx += 1;
			self.next_global_idx - 1
		});
		let node_ref = ValType::Ref(self.node_ref(false));
		let params = vec![node_ref, node_ref, ValType::I32, ValType::I32, ValType::I32, ValType::I32];
		let locals = vec![self.parts_ref(), self.parts_ref(), ValType::F64, ValType::F64, ValType::I32];
		self.runtime_function(UNCERTAIN_ORDER, params, vec![ValType::I32], locals, |s, f| {
			let (a, b, op, mode, pointer, length) = (0, 1, 2, 3, 4, 5);
			let (a_parts, b_parts, x, y, certain) = (6, 7, 8, 9, 10);
			// x op y, op in ORDERINGS' order: the strict order, or equal for <= and >=
			let compare = |f: &mut Function| Self::emit_list(f, &[
				I::LocalGet(x), I::LocalGet(y), I::F64Gt, I::LocalGet(x), I::LocalGet(y), I::F64Lt, I::LocalGet(op), I::I32Const(1), I::I32And, I::Select,
				I::LocalGet(x), I::LocalGet(y), I::F64Eq, I::LocalGet(op), I::I32Const(2), I::I32GeU, I::I32And, I::I32Or,
			]);
			let is_less = [I::LocalGet(op), I::I32Const(1), I::I32And, I::I32Eqz];
			// certainly: a's far end against b's near end (a.high < b.low); possibly: the other ends
			let compare_ends = |f: &mut Function, [a_less, a_greater, b_less, b_greater]: [i32; 4]| {
				for (parts, (when_less, when_greater), end) in [(a_parts, (a_less, a_greater), x), (b_parts, (b_less, b_greater), y)] {
					Self::emit_list(f, &s.part(parts, when_less));
					Self::emit_list(f, &s.part(parts, when_greater));
					Self::emit_list(f, &is_less);
					Self::emit_list(f, &[I::Select, I::LocalSet(end)]);
				}
				compare(f);
			};

			s.is_uncertain(f, a);
			s.is_uncertain(f, b);
			Self::emit_list(f, &[I::I32Or, I::I32Eqz, I::If(BlockType::Empty), I::LocalGet(a), I::LocalGet(b)]);
			s.call(f, NODE_ORDER);
			Self::emit_list(f, &[I::F64ConvertI32S, I::LocalSet(x), I::F64Const(0.0.into()), I::LocalSet(y)]);
			compare(f);
			Self::emit_list(f, &[I::Return, I::End]);
			for (node, parts) in [(a, a_parts), (b, b_parts)] {
				f.instruction(&I::LocalGet(node));
				s.call(f, UNCERTAIN_PARTS);
				f.instruction(&I::LocalSet(parts));
			}
			compare_ends(f, [HIGH, LOW, LOW, HIGH]);
			f.instruction(&I::LocalTee(certain));
			Self::emit_list(f, &[I::LocalGet(mode), I::I32Const(CertaintyMode::Certainly as i32), I::I32Eq, I::BrIf(0), I::Drop]);
			compare_ends(f, [LOW, HIGH, HIGH, LOW]);
			Self::emit_list(f, &[I::LocalGet(mode), I::I32Const(CertaintyMode::Possibly as i32), I::I32Eq, I::BrIf(0)]);
			match (host_warn, warned) {
				// bare: an overlap (possible, not certain) warns once
				(Some(host_warn), Some(warned)) => {
					Self::emit_list(f, &[I::LocalGet(certain), I::I32Eqz, I::I32And, I::GlobalGet(warned), I::I32Eqz, I::I32And, I::If(BlockType::Empty)]);
					Self::emit_list(f, &[I::I32Const(1), I::GlobalSet(warned), I::LocalGet(pointer), I::LocalGet(length), I::Call(host_warn), I::End]);
				}
				_ => {
					f.instruction(&I::Drop);
				}
			}
			f.instruction(&I::LocalGet(certain));
		});
	}
}
