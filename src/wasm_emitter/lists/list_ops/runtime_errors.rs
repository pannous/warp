//! Runtime errors: their names and messages, the functions that trap with them, index checks
use super::*;
use crate::lowering::data_items::Place;

pub(super) use crate::abi::traps::INDEX_NOT_INTEGRAL;
pub(in crate::wasm_emitter) use crate::abi::traps::INT_NOT_WHOLE;
pub(in crate::wasm_emitter) use crate::abi::traps::NOT_A_NUMBER;
pub(in crate::wasm_emitter) use crate::abi::traps::EMPTY_PLUS_TEXT;
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


pub use crate::abi::traps::runtime_error_class;

pub use crate::abi::traps::runtime_error_message;

pub use crate::abi::traps::INDEX_OUT_OF_RANGE_OF;
/// index_out_of_range_in(list, index): index_out_of_range_of a cons list, counting its cells
pub(super) const INDEX_OUT_OF_RANGE_IN: &str = "index_out_of_range_in";

pub use crate::abi::RETURNED_ERROR;

pub use crate::abi::traps::DIVIDE_BY_ZERO;

pub(in crate::wasm_emitter) use crate::abi::traps::NOT_JOINABLE;

pub(in crate::wasm_emitter) use crate::abi::traps::ERROR_IN_DATA;
pub(in crate::wasm_emitter) use crate::abi::traps::ERROR_IN_FIELD;

pub(super) use crate::abi::traps::NOT_A_MAP;

pub use crate::abi::traps::RUNTIME_ERRORS;

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
			let function =
				self.runtime_function(name, vec![], vec![], vec![], |s, f| s.emit_error_body(f, error_id as i32));
			// a host word failing inside a `try` raises through them as the program's own error (host/native.rs raised)
			if self.guards_errors && name == RETURNED_ERROR {
				self.exports.export(RETURNED_ERROR, ExportKind::Func, function);
				self.trap_detail_global();
			}
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

	/// An item as written into a list or field (lowering data_items): its Node, failing with a_list_item_holds_no_error or
	/// a_field_holds_no_error when it is an Error
	pub(in crate::wasm_emitter) fn emit_data_item(&mut self, func: &mut Function, item: &Node, place: Place) {
		self.emit_node_instructions(func, item);
		let held = self.node_scratch();
		Self::emit_list(func, &[I::RefAsNonNull, I::LocalSet(held)]);
		let error = match place {
			Place::ListItem => ERROR_IN_DATA,
			Place::Field => ERROR_IN_FIELD,
		};
		self.emit_fail_as_if_error(func, held, error);
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

	/// index_out_of_range_in(list, index): index_out_of_range_of with the count of the list's items
	pub(super) fn emit_index_out_of_range_in(&mut self) {
		self.ensure_index_out_of_range_of();
		let node_ref = self.node_ref(false);
		self.runtime_function(INDEX_OUT_OF_RANGE_IN, vec![Ref(node_ref), ValType::I64], vec![], vec![], |s, f| {
			Self::emit_list(f, &[I::LocalGet(1), I::LocalGet(0)]);
			s.emit_items_length(f);
			s.call(f, INDEX_OUT_OF_RANGE_OF);
		});
	}

	/// Fail with index_out_of_range_in when the i32 condition on the stack is true (list local 0, index asked for `asked`)
	pub(super) fn emit_fail_out_of_range_if(&self, func: &mut Function, asked: u32) {
		Self::emit_list(func, &[I::If(BlockType::Empty), I::LocalGet(0), I::LocalGet(asked)]);
		self.call(func, INDEX_OUT_OF_RANGE_IN);
		func.instruction(&I::End);
	}
}
