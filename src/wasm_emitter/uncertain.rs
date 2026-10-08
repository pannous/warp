//! `x ± r` at run time (card plus-minus, notes/plus_minus.md, decision P217): an interval with its value, a $Node of
//! Kind::Uncertain whose data is the f64 array [value, low, high]. The node_* arithmetic (list_ops.rs) hands an
//! uncertain operand to uncertain_add and its siblings, which take the worst case of the endpoints: `(5 ± 1) + (2 ± 1)`
//! is 7 ± 2 and `x - x` is 0 ± 2 (no correlation)

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
use crate::uncertain::{CERTAINLY, POSSIBLY};
use wasm_encoder::*;
use Instruction as I;

/// uncertain_new(value: f64, radius: f64) -> node: value - radius .. value + radius
pub const UNCERTAIN_NEW: &str = "uncertain_new";
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
const UNCERTAIN_FUNCTIONS: [&str; 4] = [UNCERTAIN_NEW, UNCERTAIN_PARTS, UNCERTAIN_FIELD, UNCERTAIN_SIMILAR];
const VALUE: i32 = 0;
const LOW: i32 = 1;
const HIGH: i32 = 2;
const RADIUS_FIELD: i32 = 3;
/// The fields of an uncertain value, by their index in the parts (RADIUS_FIELD computed): `x.value`, `x.uncertainty`
const UNCERTAIN_FIELDS: [(i32, &[&str]); 4] = [(VALUE, &["value"]), (LOW, &["low"]), (HIGH, &["high"]), (RADIUS_FIELD, &["uncertainty"])];
const PLUS_MINUS_SEPARATOR: &str = " ± ";
/// More decimals than this, or a value of MAX_SCALED units of its place or more, shows as float_text
const MAX_DECIMALS: i32 = 17;
const MAX_SCALED: f64 = 1e17;
/// Both numbers of at most 17 digits with sign and point, and the separator
const TEXT_BYTES: i32 = 64;

/// A program that makes uncertain values gets the whole family: any arithmetic may meet one
pub fn add_dependencies(required: &mut std::collections::HashSet<&'static str>) {
	if required.contains(UNCERTAIN_NEW) {
		required.extend(UNCERTAIN_FUNCTIONS);
		required.extend(UNCERTAIN_ARITHMETIC);
		required.extend([TEXT_AS_FLOAT, NUMBERS_SIMILAR, NEGATIVE_UNCERTAINTY, "new_float", UNCERTAIN_ORDER, NODE_ORDER]);
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
		self.emit_uncertain_field();
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

	/// Push the ± part of the parts in `parts`: how far the interval reaches from the value, on its farther side
	fn push_radius(&self, func: &mut Function, parts: u32) {
		Self::emit_list(func, &self.part(parts, HIGH));
		Self::emit_list(func, &self.part(parts, VALUE));
		func.instruction(&I::F64Sub);
		Self::emit_list(func, &self.part(parts, VALUE));
		Self::emit_list(func, &self.part(parts, LOW));
		Self::emit_list(func, &[I::F64Sub, I::F64Max]);
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
			// scale: the power of ten that puts the ± part in [10, 100), its 2 significant digits before the point
			let scaled_radius = [I::LocalGet(radius), I::LocalGet(scale), I::F64Mul];
			Self::emit_list(f, &[I::F64Const(1.0.into()), I::LocalSet(scale)]);
			let mut too_small = scaled_radius.to_vec();
			too_small.extend([I::F64Const(10.0.into()), I::F64Lt, I::LocalGet(decimals), I::I32Const(MAX_DECIMALS), I::I32LeS, I::I32And]);
			while_true(f, &too_small, &[
				I::LocalGet(scale), I::F64Const(10.0.into()), I::F64Mul, I::LocalSet(scale),
				I::LocalGet(decimals), I::I32Const(1), I::I32Add, I::LocalSet(decimals),
			]);
			let mut too_large = scaled_radius.to_vec();
			too_large.extend([I::F64Const(100.0.into()), I::F64Ge]);
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
