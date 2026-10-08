//! An instance passed to a function is shared, as in Python or JavaScript (P200, card instance-field). Instances are
//! values here, so a function that changes a field of a parameter (`q.x = 7`, `q.items.add(v)`, or passes it on to a
//! function or method that does) gives the changed parameters back with its value, `[value, q]`, and a call stores
//! them into the variables it passed: `f(p)` becomes `(f·shared = f(p); p = f·shared#2; f·shared#1)`. Giving the
//! parameter a new value (`q = Point(9)`) stays inside the function. Only functions mentioned solely in direct calls
//! are changed: one passed as a value keeps its plain result. Methods give their object back already (class_methods.rs).

use crate::analyzer::parameter_symbol;
use crate::class_methods::{changes_fields_of, RECEIVER};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashMap;

const SHARED_SUFFIX: &str = "·shared";
const GIVEN_SUFFIX: &str = "·given";
const RETURN_WORD: &str = "return";

/// The positions of the parameters each function changes, in order
type Changed = HashMap<String, Vec<usize>>;

struct Definition<'a> {
	name: String,
	parameters: Vec<Option<String>>,
	body: &'a Node,
}

impl Definition<'_> {
	fn is_method(&self) -> bool {
		matches!(self.parameters.first(), Some(Some(name)) if name == RECEIVER)
	}
}

pub fn lower(program: Node) -> Node {
	let mut changed = changed_parameters(&program);
	changed.retain(|_, positions| !positions.is_empty());
	let definitions = definitions(&program);
	changed.retain(|name, _| !definitions.iter().any(|definition| &definition.name == name && definition.is_method()));
	if changed.is_empty() {
		return program;
	}
	share(program, &changed)
}

/// The parameters changed by each function, grown until no function adds one: a function passing its parameter to a
/// position another one changes changes it too
fn changed_parameters(program: &Node) -> Changed {
	let definitions: Vec<Definition> = definitions(program).into_iter().filter(|definition| only_called(program, &definition.name)).collect();
	let mut changed = Changed::new();
	loop {
		let mut grew = false;
		for definition in &definitions {
			for (position, parameter) in definition.parameters.iter().enumerate() {
				let Some(parameter) = parameter else { continue };
				let known = changed.get(&definition.name).is_some_and(|positions| positions.contains(&position));
				if !known && (changes_fields_of(definition.body, parameter) || passes_on_to_change(definition.body, parameter, &changed)) {
					changed.entry(definition.name.clone()).or_default().push(position);
					grew = true;
				}
			}
		}
		if !grew {
			return changed;
		}
	}
}

/// The function definitions `(f a b) := body` of the program, nested ones too
fn definitions(program: &Node) -> Vec<Definition<'_>> {
	let mut found = vec![];
	program.visit(&mut |part| {
		if let Some((name, parameters, body)) = definition_parts(part) {
			found.push(Definition { name, parameters: parameters.iter().map(parameter_symbol).collect(), body });
		}
	});
	found
}

fn definition_parts(node: &Node) -> Option<(String, &[Node], &Node)> {
	let Node::Key(head, Op::Define | Op::Assign, body) = node.drop_meta() else { return None };
	let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
	let Node::Symbol(name) = items.first()?.drop_meta() else { return None };
	Some((name.clone(), &items[1..], body))
}

/// Is every mention of the name the head of a call or definition: no function value that callers could not see change
fn only_called(program: &Node, name: &str) -> bool {
	let (mut mentions, mut heads) = (0, 0);
	program.visit(&mut |part| match part {
		Node::Symbol(symbol) if symbol == name => mentions += 1,
		Node::List(items, Bracket::Round, _) if is_call_of(items, name) => heads += 1,
		_ => {}
	});
	mentions == heads
}

fn is_call_of(items: &[Node], name: &str) -> bool {
	matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(head)) if head == name)
}

/// Does the body pass the variable to a parameter a function changes: `f(q)`, the method call `q = inc(q)`
fn passes_on_to_change(body: &Node, variable: &str, changed: &Changed) -> bool {
	let mut passes = false;
	body.visit(&mut |part| {
		let Node::List(items, Bracket::Round, _) = part else { return };
		let Some(Node::Symbol(function)) = items.first().map(Node::drop_meta) else { return };
		let Some(positions) = changed.get(function) else { return };
		passes |= positions.iter().any(|&position| matches!(items.get(position + 1).map(Node::drop_meta), Some(Node::Symbol(name)) if name == variable));
	});
	passes
}

/// Definitions giving `[value, changed parameters…]` and calls storing those back
fn share(node: Node, changed: &Changed) -> Node {
	if let Some((name, parameters, _)) = definition_parts(&node) {
		if let Some(positions) = changed.get(&name) {
			let given: Vec<Node> = positions.iter().filter_map(|&position| parameter_symbol(&parameters[position])).map(Node::Symbol).collect();
			let Node::Key(head, op, body) = node.drop_meta().clone() else { unreachable!("a definition") };
			let body = giving_parameters(share(*body, changed), &name, &given);
			return Node::Key(head, op, Box::new(body));
		}
		let Node::Key(head, op, body) = node.drop_meta().clone() else { unreachable!("a definition") };
		return Node::Key(head, op, Box::new(share(*body, changed)));
	}
	let node = node.map_children(|child| share(child, changed));
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.first().is_some_and(|head| changed.contains_key(&head.drop_meta().name())) => {
			storing_back(node.clone(), items, &changed[&items[0].drop_meta().name()])
		}
		_ => node,
	}
}

/// `given = body; [given, q]`, each `return v` in it `return [v, q]`
fn giving_parameters(body: Node, function: &str, parameters: &[Node]) -> Node {
	let given = Node::Symbol(format!("{function}{GIVEN_SUFFIX}"));
	let with_parameters = |value: Node| Node::List([vec![value], parameters.to_vec()].concat(), Bracket::Square, Separator::Space);
	let body = returning_parameters(body, &with_parameters);
	Node::List(vec![Node::Key(Box::new(given.clone()), Op::Assign, Box::new(body)), with_parameters(given)], Bracket::None, Separator::Semicolon)
}

fn returning_parameters(node: Node, with_parameters: &dyn Fn(Node) -> Node) -> Node {
	if definition_parts(&node).is_some() {
		return node;
	}
	match node.drop_meta() {
		Node::List(items, bracket, separator) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == RETURN_WORD) => {
			Node::List(vec![items[0].clone(), with_parameters(items[1].clone())], bracket.clone(), separator.clone())
		}
		_ => node.map_children(|child| returning_parameters(child, with_parameters)),
	}
}

/// `(f·shared = f(p, 1); p = f·shared#2; f·shared#1)`: each changed argument that is a variable gets its new value
fn storing_back(call: Node, items: &[Node], positions: &[usize]) -> Node {
	let function = items[0].drop_meta().name();
	let shared = Node::Symbol(format!("{function}{SHARED_SUFFIX}"));
	let item = |position: usize| Node::Key(Box::new(shared.clone()), Op::Hash, Box::new(Node::int(position as i64)));
	let stores: Vec<Node> = positions.iter().enumerate()
		.filter_map(|(index, &position)| match items.get(position + 1).map(Node::drop_meta) {
			Some(variable @ Node::Symbol(_)) => Some(Node::Key(Box::new(variable.clone()), Op::Assign, Box::new(item(index + 2)))),
			_ => None,
		})
		.collect();
	if stores.is_empty() {
		return Node::Key(Box::new(call), Op::Hash, Box::new(Node::int(1)));
	}
	let store_result = Node::Key(Box::new(shared.clone()), Op::Assign, Box::new(call));
	Node::List([vec![store_result], stores, vec![item(1)]].concat(), Bracket::Round, Separator::Semicolon)
}
