//! `nonlocal y` written by a nested function (cards nonlocal-inner, nonlocal-cells): y lives in a cell outer and its
//! nested functions share, so inner's change is outer's, and a closure that escapes outer keeps that cell (each call
//! of outer makes its own). A cell holds any value (wasm_emitter/cells.rs): `y = e` → `cell_set(y·cell, e)`,
//! `y += e` → `cell_set(y·cell, cell_get(y·cell) + e)`, a read of y → `cell_get(y·cell)`. Variables only read keep the
//! captures of wasm_emitter `refresh_enclosing_captures`.

use crate::late_binding::{changes_in, declared_nonlocals, NONLOCAL};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasm_emitter::cells::{CELL_GET, CELL_NEW, CELL_SET};

/// `y·cell`: the variable holding y's cell (no name the parser reads)
const CELL_SUFFIX: &str = "·cell";

pub fn lower(node: Node) -> Node {
	if let Some((head, op, params, body)) = definition(&node) {
		let mut body = body.clone();
		for variable in written_nonlocals(&body) {
			body = celled(body, &variable, params.contains(&variable));
		}
		return Node::Key(Box::new(head.clone()), op, Box::new(lower(body)));
	}
	match node {
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}

/// `f(a, b) := body`: its head, operator, parameter names and body
fn definition(node: &Node) -> Option<(&Node, Op, Vec<String>, &Node)> {
	let Node::Key(head, op @ (Op::Define | Op::Assign), body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, Separator::None) = head.drop_meta() else { return None };
	let (name, params) = items.split_first()?;
	matches!(name.drop_meta(), Node::Symbol(_)).then(|| (head.as_ref(), *op, params.iter().map(|param| param.drop_meta().name()).collect(), body.as_ref()))
}

/// The definitions directly in a body (not those inside them)
fn nested_definitions(node: &Node) -> Vec<&Node> {
	if definition(node).is_some() {
		return vec![node];
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => [nested_definitions(left), nested_definitions(right)].concat(),
		Node::List(items, _, _) => items.iter().flat_map(nested_definitions).collect(),
		_ => vec![],
	}
}

/// The variables of this body that a nested function declares `nonlocal` and changes
fn written_nonlocals(body: &Node) -> Vec<String> {
	let mut written: Vec<String> = vec![];
	for nested in nested_definitions(body) {
		let (_, _, _, nested_body) = definition(nested).expect("a definition");
		for variable in declared_nonlocals(nested_body) {
			if !changes_in(nested_body, &variable).is_empty() && !written.contains(&variable) {
				written.push(variable);
			}
		}
	}
	written
}

/// The body with `variable` in a cell, made first (holding the parameter's value for a parameter, else ø)
fn celled(body: Node, variable: &str, is_param: bool) -> Node {
	let cell = Node::Symbol(format!("{variable}{CELL_SUFFIX}"));
	let initial = if is_param { Node::Symbol(variable.to_string()) } else { Node::Empty };
	let made = Node::Key(Box::new(cell.clone()), Op::Assign, Box::new(call(CELL_NEW, vec![initial])));
	match through_cell(body, variable, &cell) {
		Node::List(items, bracket @ (Bracket::Curly | Bracket::None), separator @ (Separator::Semicolon | Separator::Newline)) => {
			Node::List([vec![made], items].concat(), bracket, separator)
		}
		single => Node::List(vec![made, single], Bracket::Curly, Separator::Semicolon),
	}
}

/// Reads and changes of `variable` through its cell; a nested function where it is a parameter, or its own local
/// (changed without `nonlocal`), is another variable
fn through_cell(node: Node, variable: &str, cell: &Node) -> Node {
	let is_variable = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if name == variable);
	let get = || call(CELL_GET, vec![cell.clone()]);
	let set = |value: Node| call(CELL_SET, vec![cell.clone(), value]);
	if let Some((_, _, params, body)) = definition(&node) {
		let shadows = params.iter().any(|param| param == variable)
			|| (!changes_in(body, variable).is_empty() && !declared_nonlocals(body).iter().any(|declared| declared == variable));
		if shadows {
			return node;
		}
	}
	match node {
		Node::Symbol(ref name) if name == variable => get(),
		Node::Key(keyword, Op::Colon, names) if matches!(keyword.drop_meta(), Node::Symbol(word) if word == NONLOCAL) => Node::Key(keyword, Op::Colon, names),
		Node::Key(target, Op::Assign | Op::Define, value) if is_variable(&target) => set(through_cell(*value, variable, cell)),
		Node::Key(target, op, value) if is_variable(&target) && op.is_compound_assign() => {
			set(Node::Key(Box::new(get()), op.base_op(), Box::new(through_cell(*value, variable, cell))))
		}
		Node::Key(target, op @ (Op::Inc | Op::Dec), _) if is_variable(&target) => {
			set(Node::Key(Box::new(get()), if op == Op::Inc { Op::Add } else { Op::Sub }, Box::new(Node::from(1))))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(through_cell(*left, variable, cell)), op, Box::new(through_cell(*right, variable, cell))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| through_cell(item, variable, cell)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(through_cell(*node, variable, cell)), data },
		other => other,
	}
}

fn call(word: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(word.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}
