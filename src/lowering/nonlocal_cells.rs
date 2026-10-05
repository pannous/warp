//! `nonlocal y` written by a nested function (card nonlocal-inner): y lives in a cell outer and its nested functions
//! share, so inner's change is outer's, and a closure that escapes outer keeps that cell (each call of outer makes its
//! own). The cell is a shared array of one number (shared_arrays.rs, host-held, passed by its handle): `y = e` →
//! `shared_set(y·cell, 1, e)`, `y += e` → `shared_add(y·cell, 1, e)`, a read of y → `shared_get(y·cell, 1)`
//! (`shared_setf` … for a float). Variables only read keep the captures of wasm_emitter `refresh_enclosing_captures`.
//! A text or other value changed by inner is an error until cells hold any value.

use crate::host::{SHARED_FLOAT_WORDS, SHARED_WORDS};
use crate::late_binding::{changes_in, declared_nonlocals, NONLOCAL};
use crate::node::{error, Bracket, Node, Separator};
use crate::operators::Op;

/// `y·cell`: the variable holding y's cell (no name the parser reads)
const CELL_SUFFIX: &str = "·cell";
/// The one element of a cell
const CELL_INDEX: i64 = 1;

/// The words of a cell of Ints or of floats: new, get, set, add
struct CellWords {
	get: &'static str,
	set: &'static str,
	add: &'static str,
}

const INT_CELL: CellWords = CellWords { get: SHARED_WORDS[1], set: SHARED_WORDS[2], add: SHARED_WORDS[3] };
const FLOAT_CELL: CellWords = CellWords { get: SHARED_FLOAT_WORDS[0], set: SHARED_FLOAT_WORDS[1], add: SHARED_FLOAT_WORDS[2] };

pub fn lower(node: Node) -> Result<Node, Node> {
	if let Some((head, op, params, body)) = definition(&node) {
		let mut body = body.clone();
		for variable in written_nonlocals(&body) {
			body = celled(body, &variable, params.contains(&variable))?;
		}
		return Ok(Node::Key(Box::new(head.clone()), op, Box::new(lower(body)?)));
	}
	Ok(match node {
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)?), op, Box::new(lower(*right)?)),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower).collect::<Result<_, _>>()?, bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)?), data },
		other => other,
	})
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

/// The body with `variable` in a cell, made first (holding the parameter's value for a parameter)
fn celled(body: Node, variable: &str, is_param: bool) -> Result<Node, Node> {
	let words = match assigned_kind(&body, variable) {
		AssignedKind::Float => &FLOAT_CELL,
		AssignedKind::Number => &INT_CELL,
		AssignedKind::Other(value) => {
			let written = crate::normalize::operand_text(&value);
			return Err(error(&format!("nonlocal {variable} holds {written}: a nested function can change a nonlocal number only, for now")));
		}
	};
	let cell = Node::Symbol(format!("{variable}{CELL_SUFFIX}"));
	let mut statements = vec![Node::Key(Box::new(cell.clone()), Op::Assign, Box::new(call(SHARED_WORDS[0], vec![Node::from(CELL_INDEX)])))];
	if is_param {
		statements.push(call(words.set, vec![cell.clone(), Node::from(CELL_INDEX), Node::Symbol(variable.to_string())]));
	}
	let body = through_cell(body, variable, &cell, words);
	Ok(match body {
		Node::List(items, bracket @ (Bracket::Curly | Bracket::None), separator @ (Separator::Semicolon | Separator::Newline)) => {
			Node::List([statements, items].concat(), bracket, separator)
		}
		single => Node::List([statements, vec![single]].concat(), Bracket::Curly, Separator::Semicolon),
	})
}

enum AssignedKind {
	Number,
	Float,
	Other(Node),
}

/// What the assignments of `variable` give it: a float written anywhere makes a cell of floats
fn assigned_kind(body: &Node, variable: &str) -> AssignedKind {
	let mut kind = AssignedKind::Number;
	body.visit(&mut |node| {
		let Node::Key(target, op, value) = node else { return };
		if !matches!(target.drop_meta(), Node::Symbol(name) if name == variable) || !(matches!(op, Op::Assign | Op::Define) || op.is_compound_assign()) {
			return;
		}
		match value.drop_meta() {
			Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square | Bracket::Curly, _) => kind = AssignedKind::Other(value.as_ref().clone()),
			_ if mentions_float(value) && !matches!(kind, AssignedKind::Other(_)) => kind = AssignedKind::Float,
			_ => {}
		}
	});
	kind
}

fn mentions_float(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |inner| found |= matches!(inner, Node::Number(crate::extensions::numbers::Number::Float(_))));
	found
}

/// Reads and changes of `variable` through its cell; a nested function where it is a parameter, or its own local
/// (changed without `nonlocal`), is another variable
fn through_cell(node: Node, variable: &str, cell: &Node, words: &CellWords) -> Node {
	let is_variable = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if name == variable);
	let index = || Node::from(CELL_INDEX);
	let get = || call(words.get, vec![cell.clone(), index()]);
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
		Node::Key(target, Op::Assign | Op::Define, value) if is_variable(&target) => {
			call(words.set, vec![cell.clone(), index(), through_cell(*value, variable, cell, words)])
		}
		Node::Key(target, Op::AddAssign, value) if is_variable(&target) => {
			call(words.add, vec![cell.clone(), index(), through_cell(*value, variable, cell, words)])
		}
		Node::Key(target, op, value) if is_variable(&target) && op.is_compound_assign() => {
			let changed = Node::Key(Box::new(get()), op.base_op(), Box::new(through_cell(*value, variable, cell, words)));
			call(words.set, vec![cell.clone(), index(), changed])
		}
		Node::Key(target, op @ (Op::Inc | Op::Dec), _) if is_variable(&target) => {
			call(words.add, vec![cell.clone(), index(), Node::from(if op == Op::Inc { 1 } else { -1 })])
		}
		Node::Key(left, op, right) => Node::Key(Box::new(through_cell(*left, variable, cell, words)), op, Box::new(through_cell(*right, variable, cell, words))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| through_cell(item, variable, cell, words)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(through_cell(*node, variable, cell, words)), data },
		other => other,
	}
}

fn call(word: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(word.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}
