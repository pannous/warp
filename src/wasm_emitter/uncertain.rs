//! `x ± σ` at run time (card plus-minus, notes/plus_minus.md): a $Node of Kind::Uncertain whose data is an f64 array
//! [value, source id, contribution, source id, contribution, …], the sources ordered by id. Every `a ± b` evaluated is a
//! new source; σ is the root of the summed squared contributions, so one source met twice is correlated with itself:
//! `x - x` is 0 ± 0. The node_* arithmetic (list_ops.rs) hands an uncertain operand to uncertain_add and its siblings,
//! which scale each operand's contributions by the partial derivative of the operation (linear propagation)

use super::list_ops::TEXT_AS_FLOAT;
use super::similarity::NUMBERS_SIMILAR;
use super::WasmGcEmitter;
use crate::type_kinds::{Kind, KIND_MASK};
use wasm_encoder::*;
use Instruction as I;

/// uncertain_new(value: f64, sigma: f64) -> node: a value of its own new source
pub const UNCERTAIN_NEW: &str = "uncertain_new";
/// uncertain_parts(node) -> f64 array: an uncertain value's parts, any other number as [its value] without sources
const UNCERTAIN_PARTS: &str = "uncertain_parts";
/// uncertain_combine(a parts, b parts, value, ∂/∂a, ∂/∂b) -> node: the sources of both, scaled by the derivatives
const UNCERTAIN_COMBINE: &str = "uncertain_combine";
/// uncertain_similar(a, b, tolerance) -> i32: `a ≈ b` when a - b is within its uncertainty (or a, b numbers_similar)
pub const UNCERTAIN_SIMILAR: &str = "uncertain_similar";
const UNCERTAIN_SIGMA: &str = "uncertain_sigma";
/// `5 ± -1`
pub(super) const NEGATIVE_UNCERTAINTY: &str = "an_uncertainty_must_not_be_negative";
/// The arithmetic of node_add, node_sub, node_mul, node_div with an uncertain operand, in NODE_ARITHMETIC's order
pub const UNCERTAIN_ARITHMETIC: [&str; 4] = ["uncertain_add", "uncertain_sub", "uncertain_mul", "uncertain_div"];
const UNCERTAIN_FUNCTIONS: [&str; 5] = [UNCERTAIN_NEW, UNCERTAIN_PARTS, UNCERTAIN_COMBINE, UNCERTAIN_SIMILAR, UNCERTAIN_SIGMA];

/// A program that makes uncertain values gets the whole family: any arithmetic may meet one
pub fn add_dependencies(required: &mut std::collections::HashSet<&'static str>) {
	if required.contains(UNCERTAIN_NEW) {
		required.extend(UNCERTAIN_FUNCTIONS);
		required.extend(UNCERTAIN_ARITHMETIC);
		required.extend([TEXT_AS_FLOAT, NUMBERS_SIMILAR, NEGATIVE_UNCERTAINTY]);
	}
}

impl WasmGcEmitter {
	/// After text_as_float, before the node arithmetic, which calls these; uncertain_similar comes with the similarity ops
	pub(super) fn emit_uncertain_runtime(&mut self) {
		if !self.should_emit_function(UNCERTAIN_NEW) {
			return;
		}
		self.emit_uncertain_new();
		self.emit_uncertain_parts();
		self.emit_uncertain_sigma();
		self.emit_uncertain_combine();
		for (name, op) in UNCERTAIN_ARITHMETIC.into_iter().zip(["+", "-", "*", "/"]) {
			self.emit_uncertain_arithmetic(name, op);
		}
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

	/// The uncertain node of the kind and the parts array on the stack
	fn new_uncertain_node(&self, func: &mut Function) {
		let node = self.type_manager.node_type;
		Self::emit_list(func, &[I::RefNull(HeapType::Concrete(node)), I::StructNew(node)]);
	}

	fn emit_uncertain_new(&mut self) {
		let sources = self.uncertain_sources_global();
		let (value, sigma) = (0, 1);
		let array = self.parts_array();
		let node_ref = ValType::Ref(self.node_ref(false));
		self.runtime_function(UNCERTAIN_NEW, vec![ValType::F64; 2], vec![node_ref], vec![], |s, f| {
			Self::emit_list(f, &[I::LocalGet(sigma), I::F64Const(0.0.into()), I::F64Lt]);
			s.emit_fail_if(f, NEGATIVE_UNCERTAINTY);
			Self::emit_list(f, &[
				I::GlobalGet(sources), I::F64Const(1.0.into()), I::F64Add, I::GlobalSet(sources),
				I::I64Const(Kind::Uncertain as i64),
				I::LocalGet(value), I::GlobalGet(sources), I::LocalGet(sigma), I::ArrayNewFixed { array_type_index: array, array_size: 3 },
			]);
			s.new_uncertain_node(f);
		});
	}

	/// The count of sources made so far, the id of the last one
	fn uncertain_sources_global(&mut self) -> u32 {
		let index = self.next_global_idx;
		self.globals.global(GlobalType { val_type: ValType::F64, mutable: true, shared: false }, &ConstExpr::f64_const(0.0.into()));
		self.next_global_idx += 1;
		index
	}

	fn emit_uncertain_parts(&mut self) {
		let array = self.parts_array();
		let node_ref = ValType::Ref(self.node_ref(false));
		let parts_ref = self.parts_ref();
		self.runtime_function(UNCERTAIN_PARTS, vec![node_ref], vec![parts_ref], vec![], |s, f| {
			s.is_uncertain(f, 0);
			Self::emit_list(f, &[I::If(BlockType::Empty)]);
			s.emit_field(f, 0, 1);
			Self::emit_list(f, &[I::RefCastNonNull(HeapType::Concrete(array)), I::Return, I::End, I::LocalGet(0)]);
			s.call(f, TEXT_AS_FLOAT);
			f.instruction(&I::ArrayNewFixed { array_type_index: array, array_size: 1 });
		});
	}

	/// uncertain_sigma(parts) -> f64: √(Σ contribution²)
	fn emit_uncertain_sigma(&mut self) {
		let array = self.parts_array();
		let (parts, index, sum) = (0, 1, 2);
		self.runtime_function(UNCERTAIN_SIGMA, vec![self.parts_ref()], vec![ValType::F64], vec![ValType::I32, ValType::F64], |_, f| {
			Self::emit_list(f, &[
				I::I32Const(2), I::LocalSet(index),
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
				I::LocalGet(index), I::LocalGet(parts), I::ArrayLen, I::I32GeU, I::BrIf(1),
				I::LocalGet(parts), I::LocalGet(index), I::ArrayGet(array), I::LocalGet(parts), I::LocalGet(index), I::ArrayGet(array), I::F64Mul,
				I::LocalGet(sum), I::F64Add, I::LocalSet(sum),
				I::LocalGet(index), I::I32Const(2), I::I32Add, I::LocalSet(index), I::Br(0),
				I::End, I::End,
				I::LocalGet(sum), I::F64Sqrt,
			]);
		});
	}

	/// Merges the two ordered source lists: a source of one operand only is scaled by its derivative, a source of both
	/// gets the sum of both scaled, a contribution that cancels to 0 is left out
	fn emit_uncertain_combine(&mut self) {
		let array = self.parts_array();
		let parts_ref = self.parts_ref();
		let node_ref = ValType::Ref(self.node_ref(false));
		let (a, b, value, a_slope, b_slope) = (0, 1, 2, 3, 4);
		let (merged, i, j, k, id, contribution, take_a, take_b) = (5, 6, 7, 8, 9, 10, 11, 12);
		let mut locals = vec![parts_ref];
		locals.extend([ValType::I32; 3]);
		locals.extend([ValType::F64; 2]);
		locals.extend([ValType::I32; 2]);
		let params = vec![parts_ref, parts_ref, ValType::F64, ValType::F64, ValType::F64];
		self.runtime_function(UNCERTAIN_COMBINE, params, vec![node_ref], locals, |s, f| {
			let get = |parts: u32, index: u32, offset: i32| [I::LocalGet(parts), I::LocalGet(index), I::I32Const(offset), I::I32Add, I::ArrayGet(array)];
			// this side's next source comes first or with the other's: it has one left, and the other has none or a later one
			let takes = |f: &mut Function, mine: u32, my_index: u32, other: u32, other_index: u32, flag: u32| {
				Self::emit_list(f, &[I::LocalGet(my_index), I::LocalGet(mine), I::ArrayLen, I::I32LtU, I::If(BlockType::Result(ValType::I32))]);
				Self::emit_list(f, &[I::LocalGet(other_index), I::LocalGet(other), I::ArrayLen, I::I32GeU, I::If(BlockType::Result(ValType::I32)), I::I32Const(1), I::Else]);
				Self::emit_list(f, &get(mine, my_index, 0));
				Self::emit_list(f, &get(other, other_index, 0));
				Self::emit_list(f, &[I::F64Le, I::End, I::Else, I::I32Const(0), I::End, I::LocalSet(flag)]);
			};
			let add_scaled = |f: &mut Function, side: u32, index: u32, slope: u32, flag: u32| {
				Self::emit_list(f, &[I::LocalGet(flag), I::If(BlockType::Empty)]);
				Self::emit_list(f, &get(side, index, 0));
				f.instruction(&I::LocalSet(id));
				Self::emit_list(f, &get(side, index, 1));
				Self::emit_list(f, &[I::LocalGet(slope), I::F64Mul, I::LocalGet(contribution), I::F64Add, I::LocalSet(contribution)]);
				Self::emit_list(f, &[I::LocalGet(index), I::I32Const(2), I::I32Add, I::LocalSet(index), I::End]);
			};
			let append = |f: &mut Function, local: u32| {
				Self::emit_list(f, &[I::LocalGet(merged), I::LocalGet(k), I::LocalGet(local), I::ArraySet(array), I::LocalGet(k), I::I32Const(1), I::I32Add, I::LocalSet(k)]);
			};
			// room for every source of both after the value
			Self::emit_list(f, &[
				I::LocalGet(a), I::ArrayLen, I::LocalGet(b), I::ArrayLen, I::I32Add, I::I32Const(1), I::I32Sub,
				I::ArrayNewDefault(array), I::LocalSet(merged),
				I::LocalGet(merged), I::I32Const(0), I::LocalGet(value), I::ArraySet(array),
				I::I32Const(1), I::LocalTee(i), I::LocalTee(j), I::LocalSet(k),
				I::Block(BlockType::Empty), I::Loop(BlockType::Empty),
			]);
			takes(f, a, i, b, j, take_a);
			takes(f, b, j, a, i, take_b);
			Self::emit_list(f, &[I::LocalGet(take_a), I::LocalGet(take_b), I::I32Or, I::I32Eqz, I::BrIf(1), I::F64Const(0.0.into()), I::LocalSet(contribution)]);
			add_scaled(f, a, i, a_slope, take_a);
			add_scaled(f, b, j, b_slope, take_b);
			Self::emit_list(f, &[I::LocalGet(contribution), I::F64Const(0.0.into()), I::F64Ne, I::If(BlockType::Empty)]);
			append(f, id);
			append(f, contribution);
			Self::emit_list(f, &[I::End, I::Br(0), I::End, I::End]);
			// the merged parts, cut to their length
			Self::emit_list(f, &[
				I::I64Const(Kind::Uncertain as i64),
				I::LocalGet(k), I::ArrayNewDefault(array), I::LocalTee(a),
				I::I32Const(0), I::LocalGet(merged), I::I32Const(0), I::LocalGet(k), I::ArrayCopy { array_type_index_dst: array, array_type_index_src: array },
				I::LocalGet(a),
			]);
			s.new_uncertain_node(f);
		});
	}

	/// uncertain_<op>(a, b) -> node: the value and the derivatives by each operand, `op` one of + - * /
	fn emit_uncertain_arithmetic(&mut self, name: &'static str, op: &str) {
		let array = self.parts_array();
		let parts_ref = self.parts_ref();
		let node_ref = ValType::Ref(self.node_ref(false));
		let (a_parts, b_parts, a, b, value) = (2, 3, 4, 5, 6);
		let locals = vec![parts_ref, parts_ref, ValType::F64, ValType::F64, ValType::F64];
		self.runtime_function(name, vec![node_ref, node_ref], vec![node_ref], locals, |s, f| {
			for (node, parts, number) in [(0, a_parts, a), (1, b_parts, b)] {
				f.instruction(&I::LocalGet(node));
				s.call(f, UNCERTAIN_PARTS);
				Self::emit_list(f, &[I::LocalTee(parts), I::I32Const(0), I::ArrayGet(array), I::LocalSet(number)]);
			}
			let (operation, a_slope, b_slope): (I, Vec<I>, Vec<I>) = match op {
				"+" => (I::F64Add, vec![I::F64Const(1.0.into())], vec![I::F64Const(1.0.into())]),
				"-" => (I::F64Sub, vec![I::F64Const(1.0.into())], vec![I::F64Const((-1.0).into())]),
				"*" => (I::F64Mul, vec![I::LocalGet(b)], vec![I::LocalGet(a)]),
				_ => (I::F64Div, vec![I::F64Const(1.0.into()), I::LocalGet(b), I::F64Div], vec![I::LocalGet(value), I::LocalGet(b), I::F64Div, I::F64Neg]),
			};
			Self::emit_list(f, &[I::LocalGet(a_parts), I::LocalGet(b_parts), I::LocalGet(a), I::LocalGet(b), operation, I::LocalTee(value)]);
			Self::emit_list(f, &a_slope);
			Self::emit_list(f, &b_slope);
			s.call(f, UNCERTAIN_COMBINE);
		});
	}

	/// After numbers_similar, before values_similar
	pub(super) fn emit_uncertain_similar(&mut self) {
		if !self.should_emit_function(UNCERTAIN_SIMILAR) {
			return;
		}
		let array = self.parts_array();
		let node_ref = ValType::Ref(self.node_ref(false));
		let (a, b, tolerance, difference) = (0, 1, 2, 3);
		let sub = UNCERTAIN_ARITHMETIC[1];
		self.runtime_function(UNCERTAIN_SIMILAR, vec![node_ref, node_ref, ValType::F64], vec![ValType::I32], vec![self.parts_ref()], |s, f| {
			Self::emit_list(f, &[I::LocalGet(a), I::LocalGet(b)]);
			s.call(f, sub);
			s.call(f, UNCERTAIN_PARTS);
			Self::emit_list(f, &[I::LocalTee(difference), I::I32Const(0), I::ArrayGet(array), I::F64Abs, I::LocalGet(difference)]);
			s.call(f, UNCERTAIN_SIGMA);
			f.instruction(&I::F64Le);
			for side in [a, b] {
				f.instruction(&I::LocalGet(side));
				s.call(f, UNCERTAIN_PARTS);
				Self::emit_list(f, &[I::I32Const(0), I::ArrayGet(array)]);
			}
			f.instruction(&I::LocalGet(tolerance));
			s.call(f, NUMBERS_SIMILAR);
			f.instruction(&I::I32Or);
		});
	}
}
