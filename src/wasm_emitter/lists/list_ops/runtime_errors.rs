//! Runtime errors: their names and messages, the functions that trap with them, index checks
use super::*;

/// Runtime errors trap inside a function of that name; eval reports the name as an error value
pub(super) const INDEX_NOT_INTEGRAL: &str = "index_must_be_an_integer";
/// `y times "ab"` with y = 2.5 held in a variable (a literal fraction is a compile-time type error)
pub(super) const COUNT_NOT_INTEGRAL: &str = "count_must_be_an_integer";
/// `x:int = 3; x += 0.5`: a declared int (variable or field) given a fraction at run time; a literal one is a type error
pub(in crate::wasm_emitter) const INT_NOT_WHOLE: &str = "an_int_must_be_a_whole_number";
/// `m.a * 2` with a text in m.a, where the kinds are known only at run time (NODE_ARITHMETIC)
pub(in crate::wasm_emitter) const NOT_A_NUMBER: &str = "not_a_number";
/// P65: arithmetic a text can't do on a character fails; its code point is ord(c)
pub(in crate::wasm_emitter) const CHARACTER_IS_NO_NUMBER: &str =
	"a character is not a number in arithmetic: codepoint(c) gives its code point";

/// The division in an index `n/2`, also behind the 0-based shift of `xs[n/2]` (`xs#(n/2 + 1)`)
pub(super) fn divided_index(index: &Node) -> Option<(&Node, &Node)> {
	use crate::operators::Op;
	match index.drop_meta() {
		Node::Key(dividend, Op::Div, divisor) => Some((dividend, divisor)),
		Node::Key(shifted, Op::Add | Op::Sub, offset) if matches!(offset.drop_meta(), Node::Number(_)) => {
			divided_index(shifted)
		}
		Node::List(items, crate::node::Bracket::Round, _) if items.len() == 1 => divided_index(&items[0]),
		_ => None,
	}
}

/// How to fix a runtime error, appended to its message: the trap knows no source position, so the fix is generic
pub(super) const WHOLE_NUMBER_FIX: &str = "fix: compute it with /! (floor division) or `… as int`";
pub(super) const RUNTIME_ERROR_FIXES: [(&str, &str); 7] = [
	(
		super::super::super::numbers::casts::INVALID_CODE_POINT,
		"a code point is 0…0x10FFFF, outside the surrogates 0xD800…0xDFFF",
	),
	(NOT_A_MAP, "keys, values and entries are a map's; a list's positions are 1..count(xs)"),
	(INDEX_NOT_INTEGRAL, WHOLE_NUMBER_FIX),
	(COUNT_NOT_INTEGRAL, WHOLE_NUMBER_FIX),
	(INT_NOT_WHOLE, WHOLE_NUMBER_FIX),
	(super::super::super::declared_values::LIST_CANNOT_HOLD, "its declared element type does not admit it (P215)"),
	(ERROR_IN_DATA, "handle it with try … else, or declare `or error`"),
];

/// The message of the runtime error trapped in the function `name`: its words, then its fix if it has one
pub fn runtime_error_message(name: &str) -> String {
	let words = name.replace('_', " ");
	match RUNTIME_ERROR_FIXES.iter().find(|(error, _)| *error == name) {
		Some((_, fix)) => format!("{words}; {fix}"),
		None => words,
	}
}

/// index_out_of_range_of(index, count): index_out_of_range with the index and the list's count as the trap detail, which
/// trap_error turns into "index out of range: 7 not in 1…3" (card index-range)
pub const INDEX_OUT_OF_RANGE_OF: &str = "index_out_of_range_of";
/// index_out_of_range_in(list, index): index_out_of_range_of a cons list, counting its cells
pub(super) const INDEX_OUT_OF_RANGE_IN: &str = "index_out_of_range_in";

/// `return error "…"` from a function that returns numbers: the run fails, the message is the trap detail
pub const RETURNED_ERROR: &str = "returned_error";

/// `a % 0`, `a rem 0`: an integer divide by zero (big_int::emit_nonzero_divisor)
pub const DIVIDE_BY_ZERO: &str = "divide_by_zero";

/// `print [f]`, `join`: an item that is no text, number or other value with a text form; the item is the trap detail
pub(in crate::wasm_emitter) const NOT_JOINABLE: &str = "not_a_joinable_item";

/// `[e 2]` of an Error e: data holds no error (decision error-optional); the Error is the trap detail
pub(in crate::wasm_emitter) const ERROR_IN_DATA: &str = "a_list_item_holds_no_error";

/// `x.keys`, `x.values`, `x.entries` of a value that is no map (card keys-value)
pub(super) const NOT_A_MAP: &str = "not_a_map";

pub const RUNTIME_ERRORS: [&str; 33] = [
	"index_out_of_range",
	INDEX_NOT_INTEGRAL,
	"invalid_number",
	"out_of_memory",
	"key_not_found",
	"float_out_of_int_range",
	"min_of_an_empty_list",
	"max_of_an_empty_list",
	"reduce_of_an_empty_list",
	"not_a_list",
	"not_a_text",
	"not_an_int",
	"non_ascii_text",
	NOT_JOINABLE,
	"empty_separator",
	"not_an_object",
	"not_comparable",
	super::super::super::functions::closures::NOT_A_FUNCTION,
	super::super::super::functions::closures::WRONG_ARGUMENT_COUNT,
	super::super::super::lists::tuple_emitter::WRONG_NUMBER_OF_VALUES,
	RETURNED_ERROR,
	"not_a_character",
	COUNT_NOT_INTEGRAL,
	NOT_A_NUMBER,
	DIVIDE_BY_ZERO,
	INT_NOT_WHOLE,
	super::super::super::declared_values::LIST_CANNOT_HOLD,
	super::super::super::numbers::uncertain::NEGATIVE_UNCERTAINTY,
	super::super::super::numbers::uncertain::INTERVAL_AND_GAUSSIAN,
	NOT_A_MAP,
	super::super::super::numbers::casts::INVALID_CODE_POINT,
	super::super::super::numbers::float_text::NOT_FINITE,
	ERROR_IN_DATA,
];

impl WasmGcEmitter {
	/// The runtime error functions in id order: the fixed ones, a missing switch case per label, the fixed-width overflows
	pub(in crate::wasm_emitter) fn runtime_error_names(&self) -> Vec<&'static str> {
		let no_case_errors = self
			.ctx
			.missing_case_labels
			.iter()
			.map(|label| &*Box::leak(format!("{}{label}", crate::switch::NO_CASE_PREFIX).into_boxed_str()));
		let overflow_errors = crate::numbers::fixed_width::FIXED_WIDTHS.iter().map(|width| width.trap);
		RUNTIME_ERRORS.into_iter().chain(no_case_errors).chain(overflow_errors).collect()
	}

	pub(in crate::wasm_emitter) fn emit_runtime_errors(&mut self) {
		if self.guards_errors {
			self.declare_error_catching(); // the error functions throw while a `try` runs
		}
		for (error_id, name) in self.runtime_error_names().into_iter().enumerate() {
			self.runtime_function(name, vec![], vec![], vec![], |s, f| s.emit_error_body(f, error_id as i32));
		}
	}

	/// Unconditional runtime error; the stack after it is unreachable, so it fits any expected type
	pub(in crate::wasm_emitter) fn emit_runtime_error(&mut self, func: &mut Function, error: &'static str) {
		self.emit_call(func, error);
		func.instruction(&I::Unreachable);
	}

	/// Fail with the Error in Node local `local` as it is (its own message, the trap detail), when it holds one
	pub(in crate::wasm_emitter) fn emit_fail_if_error(&mut self, func: &mut Function, local: u32) {
		self.emit_fail_as_if_error(func, local, RETURNED_ERROR);
	}

	/// Fail with the runtime error `error` when Node local `local` holds an Error, the Error its trap detail
	pub(in crate::wasm_emitter) fn emit_fail_as_if_error(
		&mut self,
		func: &mut Function,
		local: u32,
		error: &'static str,
	) {
		let trap_detail = self.trap_detail_global();
		self.emit_field(func, local, 0);
		Self::emit_kind_test(func, Kind::Error as i64, I::I64Eq);
		Self::emit_list(func, &[I::If(BlockType::Empty), I::LocalGet(local), I::GlobalSet(trap_detail)]);
		self.call(func, error);
		func.instruction(&I::End);
	}

	/// A list item as written (lowering data_items): its Node, failing with a_list_item_holds_no_error when it is an Error
	pub(in crate::wasm_emitter) fn emit_data_item(&mut self, func: &mut Function, item: &Node) {
		self.emit_node_instructions(func, item);
		let held = self.node_scratch();
		Self::emit_list(func, &[I::RefAsNonNull, I::LocalSet(held)]);
		self.emit_fail_as_if_error(func, held, ERROR_IN_DATA);
		Self::emit_list(func, &[I::LocalGet(held), I::RefAsNonNull]);
	}

	/// Trap when the i32 condition on the stack is true
	pub(in crate::wasm_emitter) fn emit_fail_if(&self, func: &mut Function, error: &'static str) {
		func.instruction(&I::If(BlockType::Empty));
		self.call(func, error);
		func.instruction(&I::End);
	}

	/// An index held in a variable (`m = n/2; xs[m]`, `xs[0:m]`) traps at run time when the Int in i64 local `index` is a
	/// ratio, as `xs[n/2]` does: a value outside the fixnum range is a handle into the number heap (big_int.rs)
	pub(in crate::wasm_emitter) fn emit_require_integral(&self, func: &mut Function, index: u32) {
		self.emit_require_whole(func, index, INDEX_NOT_INTEGRAL);
	}

	/// Trap with `error` when the Int in i64 local `local` is a ratio
	pub(in crate::wasm_emitter) fn emit_require_whole(&self, func: &mut Function, local: u32, error: &'static str) {
		if !self.int_runtime() {
			return; // without the big-int runtime an Int is always a plain i64
		}
		let fixnum_offset = super::super::super::numbers::big_int::FIXNUM_OFFSET;
		#[rustfmt::skip]
		Self::emit_list(func, &[
			I::LocalGet(local), I::I64Const(fixnum_offset), I::I64Add, I::I64Const(0), I::I64LtS,
			I::If(BlockType::Empty),
				I::LocalGet(local),
		]);
		self.call(func, "is_ratio");
		self.emit_fail_if(func, error);
		func.instruction(&I::End);
	}

	/// `xs[n/2]`: an index that divides traps unless the division is exact; floor division (`n /! 2`) or `n/2 as int` is hinted
	pub(in crate::wasm_emitter) fn emit_integral_index_check(&mut self, func: &mut Function, index: &Node) {
		let Some((dividend, divisor)) = divided_index(index) else {
			return;
		};
		let (dividend_text, divisor_text) =
			(crate::normalize::operand_text(dividend), crate::normalize::operand_text(divisor));
		crate::normalize::set_position_of(index);
		crate::normalize::hint(
			&format!("{dividend_text}/{divisor_text}"),
			&format!("{dividend_text} /! {divisor_text}"),
			"an index must be an integer: floor division, or `as int`",
		);
		let remainder = Node::Key(Box::new(dividend.clone()), crate::operators::Op::Mod, Box::new(divisor.clone()));
		self.emit_numeric_value(func, &remainder);
		Self::emit_list(func, &[I::I64Const(0), I::I64Ne]);
		self.emit_fail_if(func, INDEX_NOT_INTEGRAL);
	}
}

impl WasmGcEmitter {
	/// index_out_of_range_of(index, count): leave `index:count` as the trap detail and fail with index_out_of_range; emitted
	/// once, before the first runtime function that calls it
	pub(in crate::wasm_emitter) fn ensure_index_out_of_range_of(&mut self) {
		if self.ctx.func_registry.contains(INDEX_OUT_OF_RANGE_OF) {
			return;
		}
		let trap_detail = self.trap_detail_global();
		self.runtime_function(INDEX_OUT_OF_RANGE_OF, vec![ValType::I64; 2], vec![], vec![], |s, f| {
			for number in [0, 1] {
				f.instruction(&I::LocalGet(number));
				s.call(f, "new_int");
			}
			f.instruction(&I::I64Const(crate::operators::op_to_code(&crate::operators::Op::Colon)));
			s.call(f, "new_key");
			f.instruction(&I::GlobalSet(trap_detail));
			s.call(f, "index_out_of_range");
		});
	}

	/// index_out_of_range_in(list, index): index_out_of_range_of with the count of the list's cells
	pub(super) fn emit_index_out_of_range_in(&mut self) {
		self.ensure_index_out_of_range_of();
		let (node_ref, node_ref_nullable) = (self.node_ref(false), self.node_ref(true));
		self.runtime_function(
			INDEX_OUT_OF_RANGE_IN,
			vec![Ref(node_ref), ValType::I64],
			vec![],
			vec![ValType::I64, Ref(node_ref_nullable)],
			|s, f| {
				let (count, current) = (2, 3);
				s.emit_field(f, 0, 0);
				Self::emit_kind_test(f, Kind::Empty as i64, I::I64Ne);
				Self::emit_list(f, &[I::If(BlockType::Empty), I::LocalGet(0), I::LocalSet(current)]);
				s.emit_cell_walk(f, current, |f| {
					Self::emit_list(f, &[I::LocalGet(count), I::I64Const(1), I::I64Add, I::LocalSet(count)])
				});
				Self::emit_list(f, &[I::End, I::LocalGet(1), I::LocalGet(count)]);
				s.call(f, INDEX_OUT_OF_RANGE_OF);
			},
		);
	}

	/// Fail with index_out_of_range_in when the i32 condition on the stack is true (list local 0, index asked for `asked`)
	pub(super) fn emit_fail_out_of_range_if(&self, func: &mut Function, asked: u32) {
		Self::emit_list(func, &[I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(asked)]);
		self.call(func, INDEX_OUT_OF_RANGE_IN);
		func.instruction(&I::End);
	}
}
