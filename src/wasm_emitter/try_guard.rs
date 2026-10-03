//! `try X else Y` catches a runtime error however deep inside X it happens (user decision #34), with the wasm
//! exception-handling proposal: every runtime error function (`index_out_of_range`, …) throws the tag `wasp_error`
//! with its error id while a `try` is running, and traps as before otherwise, so an uncaught error keeps its message.
//! `ran_without_error{statement}` runs the statement inside a `try_table` catching that tag: 1 when it finished, 0 when
//! an error was thrown. The id of the caught error stays in the global `caught_error` for a later `try … else e => …`.

use super::WasmGcEmitter;
use crate::node::Node;
use wasm_encoder::*;
use Instruction as I;

/// The marker call `try X else Y` lowers to (library_words::lower_try): `ran_without_error{try_tmp = X}`
pub const RAN_WITHOUT_ERROR: &str = "ran_without_error";
const ERROR_TAG_NAME: &str = "wasp_error";
const TRY_DEPTH_GLOBAL: &str = "try_depth";
const CAUGHT_ERROR_GLOBAL: &str = "caught_error";

/// The tag thrown by runtime errors and the globals that track running `try`s
#[derive(Clone, Copy)]
pub(super) struct ErrorCatching {
	tag: u32,
	depth_global: u32,
	caught_global: u32,
}

/// Does the program contain a `try` that catches runtime errors (decided before the runtime functions are emitted)
pub(super) fn guards_errors(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if name == RAN_WITHOUT_ERROR));
	found
}

impl WasmGcEmitter {
	/// Declare the tag `wasp_error(id: i32)` and the globals `try_depth` and `caught_error`, once
	pub(super) fn declare_error_catching(&mut self) -> ErrorCatching {
		if let Some(catching) = self.error_catching {
			return catching;
		}
		let tag_type = self.type_manager.types().len();
		self.type_manager.types_mut().ty().function(vec![ValType::I32], vec![]);
		self.tags.tag(TagType { kind: TagKind::Exception, func_type_idx: tag_type });
		let mut global = |emitter: &mut Self, name: &'static str| {
			emitter.globals.global(GlobalType { val_type: ValType::I32, mutable: true, shared: false }, &ConstExpr::i32_const(0));
			emitter.extra_global_names.push((emitter.next_global_idx, name));
			emitter.next_global_idx += 1;
			emitter.next_global_idx - 1
		};
		let depth_global = global(self, TRY_DEPTH_GLOBAL);
		let caught_global = global(self, CAUGHT_ERROR_GLOBAL);
		let catching = ErrorCatching { tag: 0, depth_global, caught_global };
		self.error_catching = Some(catching);
		catching
	}

	pub(super) fn error_tag_names(&self) -> Option<NameMap> {
		self.error_catching.map(|catching| {
			let mut names = NameMap::new();
			names.append(catching.tag, ERROR_TAG_NAME);
			names
		})
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
		func.instruction(&I::GlobalSet(catching.caught_global));
		step_depth(func, I::I32Sub);
		Self::emit_list(func, &[I::I64Const(0), I::End]);
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
