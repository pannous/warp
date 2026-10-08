//! `a ≈ b` (circa, approximately; cards approximately, approximately-all, notes/approximately.md): two numbers
//! within the relative tolerance inline (numbers_similar), any other two values at run time (values_similar): a bool and
//! any value alike in truthiness, texts and characters alike in case and accents (text_fold), lists, objects and
//! instances entry by entry with ≈ again, anything else by values_equal. `a ~ b` (P211, values_rough) is the same with
//! its own tolerance, texts also without the whitespace and punctuation around them (text_rough_trim).

use super::equality::IS_META_ENTRY;
use super::text_unicode::CaseMapping;
use super::{WasmGcEmitter, IS_TRUTHY, VALUES_EQUAL};
use super::uncertain::UNCERTAIN_SIMILAR;
use super::text_builtins::{TEXT_OF, TEXT_ROUGH_TRIM};
use crate::library_words::{VALUES_ROUGH, VALUES_SIMILAR};
use crate::node::Node;
use crate::type_kinds::{Kind, BOOL_KIND, CURLY_LIST_KIND, KIND_MASK};
use wasm_encoder::*;
use Instruction as I;

/// numbers_similar(a: f64, b: f64, tolerance: f64) -> i32: |a-b| ≤ tolerance·|a| or ≤ tolerance·|b| (or a == b)
pub const NUMBERS_SIMILAR: &str = "numbers_similar";
/// text_fold(text) -> text: lower case without accents
pub const TEXT_FOLD: &str = "text_fold";
/// What values_similar calls
const VALUES_SIMILAR_NEEDS: [&str; 5] = [NUMBERS_SIMILAR, TEXT_FOLD, VALUES_EQUAL, IS_TRUTHY, IS_META_ENTRY];
const TEXT_KINDS: [Kind; 2] = [Kind::Text, Kind::Codepoint];

/// `a ≈ b` needs numbers_similar; values_similar, with its text_fold table, only when the emitter finds an operand that is
/// not a plain number (a rerun, mod.rs discovered_needs); the same for `a ~ b` and values_rough
pub fn add_dependencies(required: &mut std::collections::HashSet<&'static str>) {
	if required.contains(VALUES_SIMILAR) || required.contains(VALUES_ROUGH) {
		required.extend(VALUES_SIMILAR_NEEDS);
	}
	if required.contains(VALUES_ROUGH) {
		required.extend([TEXT_ROUGH_TRIM, TEXT_OF]);
	}
}

impl WasmGcEmitter {
	/// Push i64 1/0 for `values_similar(a, b, tolerance)` or `values_rough(a, b, tolerance)`, the function given
	pub(super) fn emit_similarity(&mut self, func: &mut Function, function: &str, left: &Node, right: &Node, tolerance: &Node) {
		let is_plain_number = |emitter: &Self, side: &Node| matches!(emitter.get_type(side), Kind::Int | Kind::Float) && !crate::analyzer::is_boolean(side, &emitter.scope);
		if is_plain_number(self, left) && is_plain_number(self, right) {
			self.emit_float_value(func, left);
			self.emit_float_value(func, right);
			self.emit_float_value(func, tolerance);
			self.emit_call(func, NUMBERS_SIMILAR);
		} else {
			self.emit_node_instructions(func, left);
			self.emit_node_instructions(func, right);
			self.emit_float_value(func, tolerance);
			let (_, function, ..) = crate::library_words::SIMILARITY_LEVELS.iter().find(|(_, known, ..)| *known == function).expect("a similarity call");
			self.emit_call(func, function);
		}
		func.instruction(&I::I64ExtendI32U);
	}

	/// After the library ops (text_fold shares text_lower's emitter) and the equality ops
	pub(super) fn emit_similarity_ops(&mut self) {
		if self.should_emit_function(NUMBERS_SIMILAR) {
			self.emit_numbers_similar();
		}
		self.emit_uncertain_similar();
		let levels = [(VALUES_SIMILAR, false), (VALUES_ROUGH, true)].map(|(name, rough)| (name, rough, self.should_emit_function(name)));
		if levels.iter().any(|(_, _, needed)| *needed) {
			self.emit_text_case(TEXT_FOLD, CaseMapping::Fold);
		}
		for (name, rough, needed) in levels {
			if needed {
				self.emit_values_similar(name, rough);
			}
		}
	}

	fn emit_numbers_similar(&mut self) {
		let (a, b, tolerance, difference) = (0, 1, 2, 3);
		let f64t = ValType::F64;
		self.runtime_function(NUMBERS_SIMILAR, vec![f64t; 3], vec![ValType::I32], vec![f64t], |_, f| {
			Self::emit_list(f, &[I::LocalGet(a), I::LocalGet(b), I::F64Sub, I::F64Abs, I::LocalSet(difference)]);
			Self::emit_list(f, &[I::LocalGet(a), I::LocalGet(b), I::F64Eq]);
			for side in [a, b] {
				Self::emit_list(f, &[I::LocalGet(difference), I::LocalGet(tolerance), I::LocalGet(side), I::F64Abs, I::F64Mul, I::F64Le, I::I32Or]);
			}
		});
	}

	/// values_similar(a: anyref, b: anyref, tolerance: f64) -> i32, or values_rough when `rough`, which trims texts
	/// before folding them. Locals 3, 4: the kinds; 7, 8, 9: the walk of emit_unordered_entries_equal; 10, 11: the payloads
	fn emit_values_similar(&mut self, name: &'static str, rough: bool) {
		let recurse = self.next_func_idx;
		let node = self.type_manager.node_type;
		let (a, b, tolerance, kind_a, kind_b, payload_a, payload_b) = (0, 1, 2, 3, 4, 10, 11);
		let tolerance_argument = [I::LocalGet(tolerance)];
		let (i64t, i32t) = (ValType::I64, ValType::I32);
		let cell = ValType::Ref(self.node_ref(true));
		let locals = vec![i64t, i64t, i32t, i32t, cell, cell, i32t, Self::any_ref(), Self::any_ref()];
		self.runtime_function(name, vec![Self::any_ref(), Self::any_ref(), ValType::F64], vec![i32t], locals, |s, f| {
			s.skip_meta_cell(f, a, b, recurse, &tolerance_argument);
			s.skip_meta_cell(f, b, a, recurse, &tolerance_argument);
			let call_with_tolerance = |s: &Self, f: &mut Function, name: &str| {
				f.instruction(&I::LocalGet(tolerance));
				s.call(f, name);
			};
			let numbers_similar = |s: &Self, f: &mut Function, first: u32, second: u32| {
				s.number_as_f64(f, first);
				s.number_as_f64(f, second);
				call_with_tolerance(s, f, NUMBERS_SIMILAR);
				f.instruction(&I::Return);
			};
			Self::test(f, a, HeapType::Concrete(node));
			Self::test(f, b, HeapType::Concrete(node));
			Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty)]);
			for (from, to) in [(a, kind_a), (b, kind_b)] {
				Self::field(f, from, node, 0);
				f.instruction(&I::LocalSet(to));
			}
			// a bool and any value: alike in truthiness
			Self::emit_list(f, &[I::LocalGet(kind_a), I::I64Const(BOOL_KIND), I::I64Eq, I::LocalGet(kind_b), I::I64Const(BOOL_KIND), I::I64Eq, I::I32Or]);
			f.instruction(&I::If(BlockType::Empty));
			for side in [a, b] {
				f.instruction(&I::LocalGet(side));
				s.call(f, IS_TRUTHY);
			}
			Self::emit_list(f, &[I::I32Eq, I::Return, I::End]);
			// an uncertain value: alike within the uncertainty of the difference
			if s.should_emit_function(UNCERTAIN_SIMILAR) {
				for kind in [kind_a, kind_b] {
					Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(KIND_MASK), I::I64And, I::I64Const(Kind::Uncertain as i64), I::I64Eq]);
				}
				Self::emit_list(f, &[I::I32Or, I::If(BlockType::Empty)]);
				for side in [a, b] {
					Self::emit_list(f, &[I::LocalGet(side), I::RefCastNonNull(HeapType::Concrete(node))]);
				}
				call_with_tolerance(s, f, UNCERTAIN_SIMILAR);
				Self::emit_list(f, &[I::Return, I::End]);
			}
			Self::is_number_kind(f, kind_a);
			Self::is_number_kind(f, kind_b);
			Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty)]);
			for (from, to) in [(a, payload_a), (b, payload_b)] {
				Self::field(f, from, node, 1);
				f.instruction(&I::LocalSet(to));
			}
			numbers_similar(s, f, payload_a, payload_b);
			f.instruction(&I::End);
			// texts and characters: alike folded
			for kind in [kind_a, kind_b] {
				for (index, text_kind) in TEXT_KINDS.iter().enumerate() {
					Self::emit_list(f, &[I::LocalGet(kind), I::I64Const(KIND_MASK), I::I64And, I::I64Const(*text_kind as i64), I::I64Eq]);
					if index > 0 {
						f.instruction(&I::I32Or);
					}
				}
			}
			Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty)]);
			for side in [a, b] {
				Self::emit_list(f, &[I::LocalGet(side), I::RefCastNonNull(HeapType::Concrete(node))]);
				if rough {
					s.call(f, TEXT_OF);
					s.call(f, TEXT_ROUGH_TRIM);
				}
				s.call(f, TEXT_FOLD);
			}
			s.call(f, VALUES_EQUAL);
			Self::emit_list(f, &[I::Return, I::End]);
			Self::emit_list(f, &[I::LocalGet(kind_a), I::LocalGet(kind_b), I::I64Ne]);
			Self::return_if(f, 0);
			// `{…}` objects are maps: their entries may come in any order
			Self::emit_list(f, &[I::LocalGet(kind_a), I::I64Const(CURLY_LIST_KIND), I::I64Eq, I::If(BlockType::Empty)]);
			Self::emit_unordered_entries_equal(f, node, recurse, s.func_index(IS_META_ENTRY), &tolerance_argument);
			f.instruction(&I::End);
			for field_index in [1, 2] {
				Self::field(f, a, node, field_index);
				Self::field(f, b, node, field_index);
				Self::emit_list(f, &[I::LocalGet(tolerance), I::Call(recurse)]);
			}
			Self::emit_list(f, &[I::I32And, I::Return, I::End]);
			// payloads: numbers within the tolerance, anything else equal
			s.is_number_payload(f, a);
			s.is_number_payload(f, b);
			Self::emit_list(f, &[I::I32And, I::If(BlockType::Empty)]);
			numbers_similar(s, f, a, b);
			f.instruction(&I::End);
			Self::emit_list(f, &[I::LocalGet(a), I::LocalGet(b)]);
			s.call(f, VALUES_EQUAL);
		});
	}
}
