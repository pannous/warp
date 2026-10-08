//! Undo history of a signal (card web-stores, notes/web_framework.md step 12): `undo x` gives x back the value it had
//! before its last change, `redo x` the one undone. A program saying either keeps x's history: after the first main-level
//! assignment of x come its two lists (the values before, the values undone) and a listener `on change x` that adds the
//! old value to the first and empties the second, except while undo or redo itself writes x.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::parse;

const UNDO_WORD: &str = "undo";
const REDO_WORD: &str = "redo";
/// the names of x's history, plain words so the templates parse: `undo_past_x`, `undo_future_x`, `undo_moving_x`
const HISTORY_TEMPLATE: &str = "undo_past_X = []; undo_future_X = []; undo_moving_X = false; on change X { if not undo_moving_X { undo_future_X = []; undo_past_X = undo_past_X + [old] } }";
const UNDO_TEMPLATE: &str = "if undo_past_X != [] { undo_moving_X = true; undo_future_X = [X] + undo_future_X; X = undo_past_X#(count(undo_past_X)); undo_past_X.pop(); undo_moving_X = false }";
const REDO_TEMPLATE: &str = "if undo_future_X != [] { undo_moving_X = true; undo_past_X = undo_past_X + [X]; X = undo_future_X#1; undo_future_X = undo_future_X[1..]; undo_moving_X = false }";
const NAME_PLACEHOLDER: &str = "X";

pub fn lower(program: Node) -> Node {
	let mut undone = vec![];
	collect_undone(&program, &mut undone);
	let taken = [UNDO_WORD, REDO_WORD].iter().any(|word| crate::soft_keywords::program_names(&program, word));
	if undone.is_empty() || taken {
		return program;
	}
	let program = with_undo_statements(program);
	let Node::List(statements, bracket @ Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) = program.drop_meta().clone() else { return program };
	let mut kept: Vec<String> = vec![];
	let mut lowered = Vec::with_capacity(statements.len());
	for statement in statements {
		let assigned = assigned_name(&statement).filter(|name| undone.contains(name) && !kept.contains(name));
		lowered.push(statement);
		if let Some(name) = assigned {
			lowered.extend(from_template(HISTORY_TEMPLATE, &name).children().iter().cloned());
			kept.push(name);
		}
	}
	Node::List(lowered, bracket, separator)
}

/// `undo x` or `redo x`: the word and the name
fn undo_parts(node: &Node) -> Option<(&str, &str)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [word, name] = items.as_slice() else { return None };
	match (word.drop_meta(), name.drop_meta()) {
		(Node::Symbol(word), Node::Symbol(name)) if word == UNDO_WORD || word == REDO_WORD => Some((word.as_str(), name.as_str())),
		_ => None,
	}
}

fn collect_undone(node: &Node, undone: &mut Vec<String>) {
	if let Some((_, name)) = undo_parts(node) {
		if !undone.iter().any(|known| known == name) {
			undone.push(name.to_string());
		}
		return;
	}
	node.children().iter().for_each(|child| collect_undone(child, undone));
}

/// Every `undo x` and `redo x` as the block that moves x through its history
fn with_undo_statements(node: Node) -> Node {
	match undo_parts(&node) {
		Some((word, name)) => from_template(if word == UNDO_WORD { UNDO_TEMPLATE } else { REDO_TEMPLATE }, name),
		None => node.map_children(with_undo_statements),
	}
}

/// `x = …` at the main level
fn assigned_name(statement: &Node) -> Option<String> {
	let Node::Key(name, Op::Assign | Op::Define, _) = statement.drop_meta() else { return None };
	match name.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		_ => None,
	}
}

fn from_template(template: &str, name: &str) -> Node {
	parse(&template.replace(NAME_PLACEHOLDER, name))
}
