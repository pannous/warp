//! Traps: the detail and position lines of a trapped run, and the runtime error a trap means
use super::super::*;

pub use crate::abi::TRAP_DETAIL;
pub use crate::abi::TRAP_DETAIL_PREFIX;

pub use crate::abi::trap_detail_line;



/// Why a unit word is an undefined variable: the static units pass left this quantity uncomputed
pub(in crate::wasm_emitter) const UNIT_AT_RUN_TIME: &str =
	" (a unit: quantities compute in functions, loops, lists, branches and print, but not in this use yet; notes/units_runtime.md)";
/// The global behind the export `trap_detail`: not a warp name, so no user global meets it
pub(in crate::wasm_emitter) const TRAP_DETAIL_GLOBAL: &str = "trap·detail";
/// The class `raise_as` raised its error as, which `catch e` gives the Error (decision error-classes); ø for a plain raise
const TRAP_CLASS_GLOBAL: &str = "trap·class";
pub use crate::abi::TRAP_POSITION;
pub use crate::abi::TRAP_POSITION_PREFIX;
/// The global behind the export `trap_position`
pub(in crate::wasm_emitter) const TRAP_POSITION_GLOBAL: &str = "trap·position";
/// Where `trap_position` stood when the latest `try` caught an error, for `raise_on` to raise it on from there
const CAUGHT_POSITION_GLOBAL: &str = "trap·caught·position";

pub(in crate::wasm_emitter) use crate::abi::traps::COLUMN_BITS;

/// A source position of the program as the i32 `trap_position` holds: the line above, the column below
pub(in crate::wasm_emitter) fn encoded_position(position: &crate::compiler::meta::LineInfo) -> i32 {
	((position.line_nr as i32) << COLUMN_BITS) | position.column.min((1 << COLUMN_BITS) - 1) as i32
}

pub use crate::abi::trap_position_line;


pub(in crate::wasm_emitter) const TERNARY_BRANCHES: &str = "`condition ? then : else`";
pub(in crate::wasm_emitter) const IF_THEN: &str = "`if condition then ...`";




pub use crate::abi::traps::EMPTY_RANGE;


pub use crate::abi::traps::trap_error;

impl WasmGcEmitter {
	/// `error "…"` where a number is wanted (the branch of an Int if, analyzer::raises_error): it cannot be an Error value,
	/// so it fails the run with its message through `returned_error`, which `try` catches; true when it was one
	pub(crate) fn emit_raised_error(&mut self, func: &mut Function, node: &Node) -> bool {
		let Some(message) = returned_error_message(node) else {
			return false;
		};
		self.emit_trap_detail(func, message);
		self.emit_runtime_error(func, lists::list_ops::RETURNED_ERROR);
		true
	}

	/// The value of `return x` as the function gives it back: a Node, an f64 or an i64; `return error "…"` from a
	/// function of numbers fails the run with the message, it cannot give back an Error value
	pub(crate) fn emit_returned_value(&mut self, func: &mut Function, value: &Node) {
		if self.returns_list {
			self.emit_list_abi_value(func, value);
		} else if let Some(message) = returned_error_message(value).filter(|_| self.returns_number) {
			self.emit_trap_detail(func, message);
			self.emit_runtime_error(func, lists::list_ops::RETURNED_ERROR);
		} else if !self.returns_number {
			self.emit_node_instructions(func, value);
		} else if self.returns_float {
			self.emit_float_value(func, value);
		} else {
			self.emit_numeric_value(func, value);
		}
	}

	/// Leave `value` in the exported global `trap_detail` (declared on first use), for the runtime error that follows
	/// to name it: the runner reads it after the trap (wasm_reader::with_trap_detail)
	pub(crate) fn emit_trap_detail(&mut self, func: &mut Function, value: &Node) {
		self.trap_detail_global();
		self.emit_global_store(func, TRAP_DETAIL_GLOBAL, value);
		func.instruction(&I::Drop);
	}

	/// Leave the position of the statement being emitted in the exported global `trap_position`, for a runtime error of
	/// it to name: the runner reads it after the trap (wasm_reader::with_trap_detail). Again after a call, whose
	/// statements left theirs. A statement of a module or of generated code (meta::GENERATED_FILE) clears it: a position
	/// in another file would be read as the program's
	pub(crate) fn emit_trap_position(&mut self, func: &mut Function) {
		let Some(position) = self.statement_position.as_ref().filter(|position| position.line_nr > 0) else {
			return;
		};
		let encoded = match position.file {
			None => encoded_position(position),
			Some(_) => 0,
		};
		let global = self.trap_position_global();
		Self::emit_list(func, &[I32Const(encoded), I::GlobalSet(global)]);
	}

	/// Keep the position of the error a `try` caught (emit_restore_caught_position)
	pub(crate) fn emit_keep_caught_position(&mut self, func: &mut Function) {
		let (position, caught) = (self.trap_position_global(), self.caught_position_global());
		Self::emit_list(func, &[I::GlobalGet(position), I::GlobalSet(caught)]);
	}

	/// The position of the caught error as the one a runtime error reports
	pub(crate) fn emit_restore_caught_position(&mut self, func: &mut Function) {
		let (position, caught) = (self.trap_position_global(), self.caught_position_global());
		Self::emit_list(func, &[I::GlobalGet(caught), I::GlobalSet(position)]);
	}

	fn caught_position_global(&mut self) -> u32 {
		if let Some((index, _)) = self.ctx.user_globals.get(CAUGHT_POSITION_GLOBAL) {
			return *index;
		}
		let index = self.declare_global_of(ValType::I32, ConstExpr::i32_const(0));
		self.ctx.user_globals.insert(CAUGHT_POSITION_GLOBAL.to_string(), (index, Kind::Int));
		index
	}

	/// The index of the exported global `trap_position`, declared on first use
	fn trap_position_global(&mut self) -> u32 {
		if let Some((index, _)) = self.ctx.user_globals.get(TRAP_POSITION_GLOBAL) {
			return *index;
		}
		let index = self.declare_global_of(ValType::I32, ConstExpr::i32_const(0));
		self.exports.export(TRAP_POSITION, ExportKind::Global, index);
		self.ctx.user_globals.insert(TRAP_POSITION_GLOBAL.to_string(), (index, Kind::Int));
		index
	}

	/// Leave the class of the error being raised for `catch e` to read (trap_class_global): the class a raise names (a
	/// text: the class of that name) or computes (`error_class(e)`), ø for none
	pub(crate) fn emit_trap_class(&mut self, func: &mut Function, class: &Node) {
		let index = self.trap_class_global();
		match class.drop_meta() {
			Node::Text(name) => self.emit_symbol_node(func, name),
			computed => self.emit_node_instructions(func, computed),
		}
		func.instruction(&I::GlobalSet(index));
	}

	pub(crate) fn trap_class_global(&mut self) -> u32 {
		if let Some((index, _)) = self.ctx.user_globals.get(TRAP_CLASS_GLOBAL) {
			return *index;
		}
		let index = self.declare_mutable_global(Kind::Empty);
		self.ctx.user_globals.insert(TRAP_CLASS_GLOBAL.to_string(), (index, Kind::Empty));
		index
	}

	/// The index of the exported global `trap_detail`, declared on first use
	pub(crate) fn trap_detail_global(&mut self) -> u32 {
		if let Some((index, _)) = self.ctx.user_globals.get(TRAP_DETAIL_GLOBAL) {
			return *index;
		}
		let index = self.declare_mutable_global(Kind::Empty);
		self.exports.export(TRAP_DETAIL, ExportKind::Global, index);
		self.ctx.user_globals.insert(TRAP_DETAIL_GLOBAL.to_string(), (index, Kind::Empty));
		index
	}
}
