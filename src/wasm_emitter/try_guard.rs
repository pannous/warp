//! `try X else Y` catches a runtime error however deep inside X it happens (user decision #34), with the wasm
//! exception-handling proposal: every runtime error function (`index_out_of_range`, …) throws the tag `warp_error`
//! with its error id while a `try` is running, and traps as before otherwise, so an uncaught error keeps its message.
//! `ran_without_error{statement}` runs the statement inside a `try_table` catching that tag: 1 when it finished, 0 when
//! an error was thrown. The id of the caught error is dropped: the user decided against naming it (only
//! `try X else Y`). A jump out of a `try` (return, break, continue) lowers the depth first
//! (`emit_leave_tries`).

use super::WasmGcEmitter;
use crate::node::Node;
use wasm_encoder::*;
use Instruction as I;

/// The marker call `try X else Y` lowers to (library_words::lower_try): `ran_without_error{try_tmp = X}`
pub const RAN_WITHOUT_ERROR: &str = "ran_without_error";
const ERROR_TAG_NAME: &str = "warp_error";
const TRY_DEPTH_GLOBAL: &str = "try_depth";
const CAUGHT_ERROR_GLOBAL: &str = "caught_error_id";
/// `caught_error(finished, value)` (P67, library_words::lower_try_binding): the Error `catch e` binds: value when the
/// guarded statement finished (it gave back an Error), else the runtime error the latest `try` caught, by its id
pub const CAUGHT_ERROR: &str = "caught_error";
/// `ran_without_abort(i, {statement})` (scoped_handlers.rs, notes/effect_handlers.md step 3): 1 when the statement
/// finished, 0 when the block handler i aborted it with `break value`
pub const RAN_WITHOUT_ABORT: &str = "ran_without_abort";
/// `abort_to(i)`: block handler i ends its block, however deep the emit was
pub const ABORT_TO: &str = "abort_to";
const ABORT_TAG_NAME: &str = "warp_abort";
const ABORT_PAYLOAD_GLOBAL: &str = "aborting_handler";

/// The tag an aborting handler throws, apart from warp_error so no `try` catches it, and the global holding its payload
/// while a block checks whether the abort is its own
#[derive(Clone, Copy)]
pub(super) struct AbortCatching {
	tag: u32,
	payload_global: u32,
}

/// The tag thrown by runtime errors and the globals that track running `try`s
#[derive(Clone, Copy)]
pub(super) struct ErrorCatching {
	tag: u32,
	depth_global: u32,
	/// the id of the error the latest `try` caught, which `catch e` reads (caught_error)
	caught_global: u32,
}

/// Does the program contain a `try` that catches runtime errors (decided before the runtime functions are emitted)
pub(super) fn guards_errors(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if name == RAN_WITHOUT_ERROR));
	found
}

impl WasmGcEmitter {
	/// Declare the tag `warp_error(id: i32)` and the global `try_depth`, once
	pub(super) fn declare_error_catching(&mut self) -> ErrorCatching {
		if let Some(catching) = self.error_catching {
			return catching;
		}
		let tag = self.declare_i32_tag();
		self.globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(0));
		self.extra_global_names.push((self.next_global_idx, TRY_DEPTH_GLOBAL));
		let depth_global = self.next_global_idx;
		self.next_global_idx += 1;
		self.globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(-1));
		self.extra_global_names.push((self.next_global_idx, CAUGHT_ERROR_GLOBAL));
		let caught_global = self.next_global_idx;
		self.next_global_idx += 1;
		let catching = ErrorCatching { tag, depth_global, caught_global };
		self.error_catching = Some(catching);
		catching
	}

	/// A new exception tag carrying one i32
	fn declare_i32_tag(&mut self) -> u32 {
		let tag = self.tags.len();
		let tag_type = self.type_manager.types().len();
		self.type_manager.types_mut().ty().function(vec![ValType::I32], vec![]);
		self.tags.tag(TagType { kind: TagKind::Exception, func_type_idx: tag_type });
		tag
	}

	/// Declare the tag `warp_abort(handler: i32)` and the global its catcher reads the payload into, once
	fn declare_abort_catching(&mut self) -> AbortCatching {
		if let Some(catching) = self.abort_catching {
			return catching;
		}
		let tag = self.declare_i32_tag();
		self.globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(0));
		self.extra_global_names.push((self.next_global_idx, ABORT_PAYLOAD_GLOBAL));
		let catching = AbortCatching { tag, payload_global: self.next_global_idx };
		self.next_global_idx += 1;
		self.abort_catching = Some(catching);
		catching
	}

	pub(super) fn error_tag_names(&self) -> Option<NameMap> {
		let tags: Vec<(u32, &str)> = self.error_catching.map(|catching| (catching.tag, ERROR_TAG_NAME)).into_iter()
			.chain(self.abort_catching.map(|catching| (catching.tag, ABORT_TAG_NAME))).collect();
		(!tags.is_empty()).then(|| {
			let mut names = NameMap::new();
			let mut tags = tags;
			tags.sort();
			tags.into_iter().for_each(|(tag, name)| names.append(tag, name));
			names
		})
	}

	/// `abort_to(i)`: throw i to the block of handler i; the stack after it is unreachable, so it fits any type
	pub(super) fn emit_abort_to(&mut self, func: &mut Function, handler: &Node) {
		let catching = self.declare_abort_catching();
		self.emit_numeric_value(func, handler);
		Self::emit_list(func, &[I::I32WrapI64, I::Throw(catching.tag), I::Unreachable]);
	}

	/// `ran_without_abort(i, {statement})` as i64: 1 when the statement finished, 0 when handler i aborted it. Another
	/// handler's abort goes on to its own block. The `try`s an abort jumped out of never ended: try_depth is put back
	/// to what it was here (an i64 temp local, analyzer collect_variables counts it)
	pub(super) fn emit_ran_without_abort(&mut self, func: &mut Function, handler: &Node, statement: &Node) {
		let catching = self.declare_abort_catching();
		let saved_depth = self.next_temp_local;
		self.next_temp_local += 1;
		let depth = self.error_catching.map(|errors| errors.depth_global);
		if let Some(depth) = depth {
			Self::emit_list(func, &[I::GlobalGet(depth), I::I64ExtendI32U, I::LocalSet(saved_depth)]);
		}
		let statement = match statement.drop_meta() {
			Node::List(items, crate::node::Bracket::Curly, _) if items.len() == 1 => &items[0],
			other => other,
		};
		self.initialize_assigned_node(func, statement);
		Self::emit_list(func, &[
			I::Block(BlockType::Result(ValType::I64)), // finished
			I::Block(BlockType::Result(ValType::I32)), // caught, with the aborting handler
			I::TryTable(BlockType::Empty, vec![Catch::One { tag: catching.tag, label: 0 }].into()),
		]);
		self.emit_discarded_statement(func, statement, Self::emit_node_instructions);
		func.instruction(&I::Drop);
		func.instruction(&I::End); // try_table
		Self::emit_list(func, &[I::I64Const(1), I::Br(1), I::End]); // caught
		func.instruction(&I::GlobalSet(catching.payload_global));
		Self::emit_list(func, &[I::GlobalGet(catching.payload_global), I::I64ExtendI32U]);
		self.emit_numeric_value(func, handler);
		Self::emit_list(func, &[I::I64Ne, I::If(BlockType::Empty), I::GlobalGet(catching.payload_global), I::Throw(catching.tag), I::End]);
		if let Some(depth) = depth {
			Self::emit_list(func, &[I::LocalGet(saved_depth), I::I32WrapI64, I::GlobalSet(depth)]);
		}
		Self::emit_list(func, &[I::I64Const(0), I::End]);
	}

	/// The body of a runtime error function: throw its id while a `try` runs, else trap (the runner names the error by
	/// the trapping function)
	pub(super) fn emit_error_body(&self, func: &mut Function, error_id: i32) {
		if let Some(catching) = self.error_catching {
			Self::emit_list(func, &[
				I::GlobalGet(catching.depth_global), I::If(BlockType::Empty),
				I::I32Const(error_id), I::Throw(catching.tag),
				I::End,
			]);
		}
		func.instruction(&I::Unreachable);
	}

	/// Before a jump out of the frames from `target_frame` on (0 for `return`): the `try`s it leaves are over, so the depth
	/// drops by their count, as their normal end would have done. Every exit path takes this, whatever its keyword.
	pub(super) fn emit_leave_tries(&self, func: &mut Function, target_frame: usize) {
		let frames = super::loop_control::open_frames(func);
		let left = frames.get(target_frame..).unwrap_or_default().iter().filter(|is_try| **is_try).count();
		let Some(catching) = self.error_catching.filter(|_| left > 0) else { return };
		Self::emit_list(func, &[I::GlobalGet(catching.depth_global), I::I32Const(left as i32), I::I32Sub, I::GlobalSet(catching.depth_global)]);
	}

	/// `ran_without_error{statement}` as i64: 1 when the statement finished, 0 when a runtime error was thrown in it.
	/// The depth goes up for the statement and back down on both paths.
	pub(super) fn emit_ran_without_error(&mut self, func: &mut Function, statement: &Node) {
		let catching = self.declare_error_catching();
		let statement = match statement.drop_meta() {
			Node::List(items, crate::node::Bracket::Curly, _) if items.len() == 1 => &items[0],
			other => other,
		};
		self.initialize_assigned_node(func, statement);
		let step_depth = |func: &mut Function, step: I<'static>| {
			Self::emit_list(func, &[I::GlobalGet(catching.depth_global), I::I32Const(1), step, I::GlobalSet(catching.depth_global)]);
		};
		step_depth(func, I::I32Add);
		Self::emit_list(func, &[
			I::Block(BlockType::Result(ValType::I64)), // finished
			I::Block(BlockType::Result(ValType::I32)), // caught, with the error id
			I::TryTable(BlockType::Empty, vec![Catch::One { tag: catching.tag, label: 0 }].into()),
		]);
		self.emit_discarded_statement(func, statement, Self::emit_node_instructions);
		func.instruction(&I::Drop);
		func.instruction(&I::End); // try_table
		step_depth(func, I::I32Sub);
		Self::emit_list(func, &[I::I64Const(1), I::Br(1), I::End]); // caught
		func.instruction(&I::GlobalSet(catching.caught_global)); // the error id, for `catch e`
		step_depth(func, I::I32Sub);
		Self::emit_list(func, &[I::I64Const(0), I::End]);
	}

	/// `caught_error(finished, value)` as a Node: value when finished, else the Error of the caught error's id: a raised
	/// value's text (the trap detail) or the runtime error's message
	pub(super) fn emit_caught_error(&mut self, func: &mut Function, finished: &Node, value: &Node) {
		let catching = self.declare_error_catching();
		let node_ref = ValType::Ref(self.node_ref(false));
		self.emit_numeric_value(func, finished);
		Self::emit_list(func, &[I::I64Eqz, I::If(BlockType::Result(node_ref))]);
		let names = self.runtime_error_names();
		for (id, name) in names.iter().enumerate() {
			Self::emit_list(func, &[I::GlobalGet(catching.caught_global), I::I32Const(id as i32), I::I32Eq, I::If(BlockType::Result(node_ref))]);
			if *name == super::list_ops::RETURNED_ERROR {
				let detail = self.trap_detail_global();
				Self::emit_list(func, &[I::GlobalGet(detail), I::RefAsNonNull]);
			} else {
				self.emit_string_call(func, &super::list_ops::runtime_error_message(name), "new_text");
			}
			self.emit_call(func, super::text_builtins::ERROR_OF);
			func.instruction(&I::Else);
		}
		self.emit_string_call(func, "error", "new_text");
		self.emit_call(func, super::text_builtins::ERROR_OF);
		names.iter().for_each(|_| { func.instruction(&I::End); });
		func.instruction(&I::Else);
		self.emit_node_instructions(func, value);
		func.instruction(&I::End);
	}

	/// A Node variable is a non-nullable local that wasm counts as set only after the block setting it: assigned inside
	/// the `try_table`, it holds ø first
	fn initialize_assigned_node(&mut self, func: &mut Function, statement: &Node) {
		let Node::Key(target, crate::operators::Op::Assign, _) = statement.drop_meta() else { return };
		let Node::Symbol(name) = target.drop_meta() else { return };
		let Some(local) = self.scope.lookup(name).filter(|local| local.kind.is_ref()).map(|local| local.position) else { return };
		self.emit_call(func, "new_empty");
		func.instruction(&I::LocalSet(local));
	}
}
