//! Nested functions in the type model. W0 has main-level functions only, so `inner` defined in the body of `outer` is
//! lifted to the main-level `outer·inner`, taking the names of outer it reads as further parameters, passed by name at
//! each call. A name inner declares `nonlocal` (or a sibling it calls does) is passed as outer's cell: the parameter
//! `y: outer·y` holds the cell itself, so inner's writes reach outer. A plain `y = 7` in inner is inner's own local,
//! as in Python and warp.

use super::{assigned_locals, cell_class, function_definition, mentions, parameter_name, statements};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const NONLOCAL_WORD: &str = "nonlocal";

struct Nested {
	name: String,
	lifted: String,
	parameters: Vec<Node>,
	body: Node,
	/// the names of outer it reads, in outer's order
	captures: Vec<String>,
	/// the captures it changes: passed as outer's cells
	references: Vec<String>,
}

/// The program with every function defined inside a main-level function's body lifted to main level
pub(super) fn lift(program: &Node, globals: &[String]) -> Result<Node, String> {
	let parts = statements(program);
	if !parts.iter().any(|statement| has_nested(statement)) {
		return Ok(program.clone());
	}
	let mut lifted = vec![];
	for statement in parts {
		match function_definition(statement) {
			Some((outer, parameters, body)) if has_nested(statement) => lifted.extend(lift_from(outer, &parameters, body, globals)?),
			_ => lifted.push(statement.clone()),
		}
	}
	// a function nested two deep is now one deep
	lift(&Node::List(lifted, Bracket::None, Separator::Semicolon), globals)
}

fn has_nested(statement: &Node) -> bool {
	function_definition(statement).is_some_and(|(_, _, body)| body_items(body).iter().any(|item| function_definition(item).is_some()))
}

fn body_items(body: &Node) -> &[Node] {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items,
		_ => &[],
	}
}

/// The lifted functions, then outer without them
fn lift_from(outer: &str, parameters: &[&Node], body: &Node, globals: &[String]) -> Result<Vec<Node>, String> {
	let (definitions, rest): (Vec<&Node>, Vec<&Node>) = body_items(body).iter().partition(|item| function_definition(item).is_some());
	let rest_body = Node::List(rest.into_iter().cloned().collect(), Bracket::Curly, Separator::Semicolon);
	let mut outer_names: Vec<String> = parameters.iter().map(|parameter| parameter_name(parameter)).collect();
	let outer_cells = assigned_locals(&rest_body, globals);
	outer_names.extend(outer_cells.iter().filter(|cell| !outer_names.contains(cell)).cloned().collect::<Vec<_>>());
	let mut nested: Vec<Nested> = definitions.iter().map(|definition| {
		let (name, parameters, body) = function_definition(definition).expect("a function definition");
		let own: Vec<String> = parameters.iter().map(|parameter| parameter_name(parameter)).collect();
		let references = nonlocal_names(body);
		let locals: Vec<String> = assigned_locals(body, globals).into_iter().filter(|local| !references.contains(local)).collect();
		let captures = outer_names.iter().filter(|name| !own.contains(name) && !locals.contains(name) && mentions(body, name)).cloned().collect();
		Nested { name: name.to_string(), lifted: cell_class(outer, name), parameters: parameters.into_iter().cloned().collect(), body: without_nonlocal(body), captures, references }
	}).collect();
	// a call of a sibling passes its captures on: the caller captures them too
	loop {
		let mut changed = false;
		for caller in 0..nested.len() {
			for callee in 0..nested.len() {
				if caller == callee || !mentions(&nested[caller].body, &nested[callee].name) {
					continue;
				}
				let (captures, references) = (nested[callee].captures.clone(), nested[callee].references.clone());
				changed |= include(&mut nested[caller].captures, captures);
				changed |= include(&mut nested[caller].references, references);
			}
		}
		if !changed {
			break;
		}
	}
	for function in &mut nested {
		function.captures.sort_by_key(|capture| outer_names.iter().position(|name| name == capture));
		if let Some(missing) = function.references.iter().find(|reference| !outer_cells.contains(reference)) {
			return Err(format!("not in W0: nonlocal {missing} of {outer}, which {outer} does not assign"));
		}
	}
	let mut lifted = vec![];
	for function in &nested {
		let captured = function.captures.iter().map(|capture| match () {
			_ if function.references.contains(capture) => typed(capture, &cell_class(outer, capture)),
			_ => parameters.iter().find(|parameter| parameter_name(parameter) == *capture).map_or_else(|| Node::Symbol(capture.clone()), |parameter| parameter.drop_meta().clone()),
		});
		let head = Node::List(std::iter::once(Node::Symbol(function.lifted.clone())).chain(function.parameters.iter().cloned()).chain(captured).collect(), Bracket::Round, Separator::None);
		lifted.push(Node::Key(Box::new(head), Op::Define, Box::new(with_lifted_calls(&function.body, &nested))));
	}
	let outer_head = Node::List(std::iter::once(Node::Symbol(outer.to_string())).chain(parameters.iter().map(|parameter| (*parameter).clone())).collect(), Bracket::Round, Separator::None);
	lifted.push(Node::Key(Box::new(outer_head), Op::Define, Box::new(with_lifted_calls(&rest_body, &nested))));
	Ok(lifted)
}

/// adds the names not yet in names; whether any was new
fn include(names: &mut Vec<String>, more: Vec<String>) -> bool {
	let before = names.len();
	for name in more {
		if !names.contains(&name) {
			names.push(name);
		}
	}
	names.len() > before
}

fn typed(name: &str, type_word: &str) -> Node {
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Colon, Box::new(Node::Symbol(type_word.to_string())))
}


/// `nonlocal y` (parsed `nonlocal: y`)
fn nonlocal_declaration(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::Key(word, Op::Colon, name) if matches!(word.drop_meta(), Node::Symbol(word) if word == NONLOCAL_WORD) => Some(name.name()),
		_ => None,
	}
}

fn nonlocal_names(body: &Node) -> Vec<String> {
	body_items(body).iter().filter_map(nonlocal_declaration).collect()
}

fn without_nonlocal(body: &Node) -> Node {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, separator) => Node::List(items.iter().filter(|item| nonlocal_declaration(item).is_none()).cloned().collect(), Bracket::Curly, separator.clone()),
		other => other.clone(),
	}
}

/// Each call of a nested function calls its lifted one, the captures appended: by name, or as the one argument
fn with_lifted_calls(node: &Node, nested: &[Nested]) -> Node {
	let node = node.clone().map_children(|child| with_lifted_calls(&child, nested));
	let Node::List(items, bracket, separator) = node.drop_meta() else { return node };
	let Some(function) = items.first().and_then(|first| nested.iter().find(|function| matches!(first.drop_meta(), Node::Symbol(name) if *name == function.name))) else { return node };
	let arguments = &items[1..];
	let parameter_count = function.parameters.len() + function.captures.len();
	let captured = function.captures.iter().map(|capture| match parameter_count {
		1 => Node::Symbol(capture.clone()),
		_ => Node::Key(Box::new(Node::Symbol(capture.clone())), Op::Assign, Box::new(Node::Symbol(capture.clone()))),
	});
	let items = std::iter::once(Node::Symbol(function.lifted.clone())).chain(arguments.iter().cloned()).chain(captured).collect();
	Node::List(items, bracket.clone(), separator.clone())
}
