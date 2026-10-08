//! `x ± r` at run time (card plus-minus, notes/plus_minus.md, decision P217): an interval with its value, a $Node of
//! Kind::Uncertain whose data is the f64 array [value, low, high]. The node_* arithmetic (list_ops.rs) hands an
//! uncertain operand to uncertain_add and its siblings, which take the worst case of the endpoints: `(5 ± 1) + (2 ± 1)`
//! is 7 ± 2 and `x - x` is 0 ± 2 (no correlation)

use super::list_ops::TEXT_AS_FLOAT;
use super::similarity::NUMBERS_SIMILAR;
use super::WasmGcEmitter;
use crate::type_kinds::{Kind, KIND_MASK};
use wasm_encoder::*;
use Instruction as I;

/// uncertain_new(value: f64, radius: f64) -> node: value - radius .. value + radius
pub const UNCERTAIN_NEW: &str = "uncertain_new";
/// uncertain_parts(node) -> f64 array: an uncertain value's [value, low, high], any other number as [n, n, n]
const UNCERTAIN_PARTS: &str = "uncertain_parts";
/// uncertain_similar(a, b, tolerance) -> i32: `a ≈ b` when the intervals overlap (or the values are numbers_similar)
pub const UNCERTAIN_SIMILAR: &str = "uncertain_similar";
/// `5 ± -1`
pub(super) const NEGATIVE_UNCERTAINTY: &str = "an_uncertainty_must_not_be_negative";
/// The arithmetic of node_add, node_sub, node_mul, node_div with an uncertain operand, in NODE_ARITHMETIC's order
pub const UNCERTAIN_ARITHMETIC: [&str; 4] = ["uncertain_add", "uncertain_sub", "uncertain_mul", "uncertain_div"];
const UNCERTAIN_FUNCTIONS: [&str; 3] = [UNCERTAIN_NEW, UNCERTAIN_PARTS, UNCERTAIN_SIMILAR];
const VALUE: i32 = 0;
const LOW: i32 = 1;
const HIGH: i32 = 2;

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
		for (name, operation) in UNCERTAIN_ARITHMETIC.into_iter().zip([I::F64Add, I::F64Sub, I::F64Mul, I::F64Div]) {
			self.emit_uncertain_arithmetic(name, operation);
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

	/// [I::LocalGet(parts), index, ArrayGet]: one part of the array in `parts`
	fn part(&self, parts: u32, index: i32) -> [I<'static>; 3] {
		[I::LocalGet(parts), I::I32Const(index), I::ArrayGet(self.parts_array())]
	}

	/// The uncertain node of the three f64 on the stack
	fn new_uncertain_node(&self, func: &mut Function) {
		let node = self.type_manager.node_type;
		Self::emit_list(func, &[
			I::ArrayNewFixed { array_type_index: self.parts_array(), array_size: 3 },
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
	fn emit_uncertain_arithmetic(&mut self, name: &'static str, operation: I<'static>) {
		let node_ref = ValType::Ref(self.node_ref(false));
		let (a, b, low, high) = (2, 3, 4, 5);
		let locals = vec![self.parts_ref(), self.parts_ref(), ValType::F64, ValType::F64];
		self.runtime_function(name, vec![node_ref, node_ref], vec![node_ref], locals, |s, f| {
			for (node, parts) in [(0, a), (1, b)] {
				f.instruction(&I::LocalGet(node));
				s.call(f, UNCERTAIN_PARTS);
				f.instruction(&I::LocalSet(parts));
			}
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
