//! `break` and `continue` (Crystal's `next`) inside `while` loops and the `for` loops lowered to them.
//!
//! A loop with jumps emits `block $break { loop { test; block $continue { body } step; br loop } }`:
//! `break` branches out of `$break`, `continue` out of `$continue` so the `for` step still runs.
//! The `for` lowering marks its step (see `mark_step`) so `continue` cannot skip it.
//! wasm labels are relative depths: a jump counts the control frames opened since its target.

use super::WasmGcEmitter;
use crate::node::Node;
use crate::operators::Op;
use wasm_encoder::{Function, Instruction};

const BREAK_WORDS: [&str; 1] = ["break"];
const CONTINUE_WORDS: [&str; 2] = ["continue", "next"];
/// `next` is also a common variable or data name: it is only a jump inside a loop
const WORDS_ONLY_IN_LOOPS: [&str; 1] = ["next"];
const STEP_MARKER: &str = "loop·step";

#[derive(Clone, Copy)]
pub(crate) struct LoopLabels {
	break_frame: usize,
	continue_frame: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Jump {
	Break,
	Continue,
}

fn jump_word(node: &Node) -> Option<(Jump, &str)> {
	let Node::Symbol(name) = node.drop_meta() else { return None };
	let word = name.as_str();
	if BREAK_WORDS.contains(&word) {
		Some((Jump::Break, word))
	} else if CONTINUE_WORDS.contains(&word) {
		Some((Jump::Continue, word))
	} else {
		None
	}
}

/// The step a `for` loop appends to its body, run after the body and after every `continue`
pub(crate) fn mark_step(step: Node) -> Node {
	Node::meta(step, Node::Symbol(STEP_MARKER.to_string()))
}

fn is_step(node: &Node) -> bool {
	matches!(node, Node::Meta { data, .. } if matches!(data.drop_meta(), Node::Symbol(marker) if marker == STEP_MARKER))
}

/// The loop body without the step a `for` lowering appended, and that step
pub(crate) fn split_step(body: &Node) -> (Node, Option<Node>) {
	match body.drop_meta() {
		Node::List(items, bracket, separator) if items.last().is_some_and(is_step) => {
			let (step, statements) = items.split_last().expect("guarded");
			(Node::List(statements.to_vec(), bracket.clone(), separator.clone()), Some(step.clone()))
		}
		_ => (body.clone(), None),
	}
}

/// Does the body jump out of its own loop (jumps of nested loops belong to those)
pub(crate) fn has_jump(body: &Node) -> bool {
	match body.drop_meta() {
		Node::Key(_, Op::Do, _) => false,
		Node::Key(left, _, right) => has_jump(left) || has_jump(right),
		Node::List(items, _, _) => items.iter().any(has_jump),
		node => jump_word(node).is_some(),
	}
}

/// Open control frames (block, loop, if, try) at the end of the code emitted so far
pub(crate) fn open_control_frames(func: &Function) -> usize {
	use wasmparser::Operator::*;
	let body = func.clone().into_raw_body();
	let reader = wasmparser::FunctionBody::new(wasmparser::BinaryReader::new(&body, 0));
	let Ok(mut operators) = reader.get_operators_reader() else { return 0 };
	let mut open = 0usize;
	while !operators.eof() {
		match operators.read() {
			Ok(Block { .. } | Loop { .. } | If { .. } | Try { .. } | TryTable { .. }) => open += 1,
			Ok(End | Delegate { .. }) => open = open.saturating_sub(1),
			Ok(_) => {}
			Err(_) => break,
		}
	}
	open
}

impl WasmGcEmitter {
	pub(crate) fn enter_loop(&mut self, break_frame: usize, continue_frame: usize) {
		self.loop_labels.push(LoopLabels { break_frame, continue_frame });
	}

	pub(crate) fn leave_loop(&mut self) {
		self.loop_labels.pop();
	}

	fn is_defined_name(&self, name: &str) -> bool {
		self.scope.lookup(name).is_some() || self.ctx.user_globals.contains_key(name) || self.ctx.user_functions.contains_key(name)
	}

	/// Emit `break`/`continue` as a branch to the innermost loop; false if the node is no jump.
	/// A jump outside any loop is a compile error, never silently a symbol.
	pub(crate) fn emit_loop_jump(&mut self, func: &mut Function, node: &Node) -> bool {
		let Some((jump, word)) = jump_word(node) else { return false };
		if self.is_defined_name(word) {
			return false;
		}
		let Some(labels) = self.loop_labels.last().copied() else {
			if WORDS_ONLY_IN_LOOPS.contains(&word) {
				return false;
			}
			self.emit_type_error(func, format!("`{word}` outside of a loop: it only leaves or skips a while/for loop body"));
			return true;
		};
		let target = if jump == Jump::Break { labels.break_frame } else { labels.continue_frame };
		let depth = open_control_frames(func) - target;
		func.instruction(&Instruction::Br(depth as u32));
		true
	}

	/// User functions have their own loops: a jump never crosses a function boundary
	pub(crate) fn take_loop_labels(&mut self) -> Vec<LoopLabels> {
		std::mem::take(&mut self.loop_labels)
	}

	pub(crate) fn restore_loop_labels(&mut self, labels: Vec<LoopLabels>) {
		self.loop_labels = labels;
	}
}
