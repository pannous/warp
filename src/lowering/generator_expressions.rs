//! Generator expressions (card generators-generator): `(x * x for x in xs)` and `(x for x in xs if x > 1)` are
//! generators. Each becomes the generator `generator·expression·1(free…) := { for x in xs { if c { yield x * x } } }`
//! and its call, so a loop or take over it runs lazily, `next` resumes it and any other use collects (generators.rs).
//! Its parameters are the variables it reads, passed when it is made. An element of several words is no generator
//! expression: `(upper w for w in words)` reads as the call `upper(w for w in words)` (comprehensions.rs).

use crate::comprehensions::{bound_names, comprehension_parts, Comprehension};
use crate::generators::{statements, symbol, symbol_name, FOR_WORD, NAME_SEPARATOR};
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;
use std::collections::BTreeSet;

const NAME: &str = "generator·expression";
const YIELDING_LOOP: &str = "for VARIABLE in SEQUENCE { yield ELEMENT }";
const FILTERED_LOOP: &str = "for VARIABLE in SEQUENCE { if CONDITION { yield ELEMENT } }";

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[FOR_WORD]) {
		return node;
	}
	let variables = bound_variables(&node);
	let mut definitions = vec![];
	let node = expressions(node, &variables, &mut definitions);
	if definitions.is_empty() {
		return node;
	}
	let items = match node {
		Node::List(items, Bracket::None | Bracket::Curly, Separator::Semicolon | Separator::Newline) => items,
		other => vec![other],
	};
	statements(definitions.into_iter().chain(items).collect(), Bracket::None)
}

/// The names the program assigns, takes as parameters or loops over
fn bound_variables(node: &Node) -> BTreeSet<String> {
	let mut variables = BTreeSet::new();
	node.visit(&mut |part| match part {
		Node::Key(target, Op::Assign | Op::Define, _) => variables.extend(bound_names(target)),
		Node::List(words, _, _) if words.first().is_some_and(|word| symbol_name(word).is_some_and(|name| name == FOR_WORD)) => {
			variables.extend(words.get(1).and_then(symbol_name).cloned());
		}
		_ => {}
	});
	variables
}

fn expressions(node: Node, variables: &BTreeSet<String>, definitions: &mut Vec<Node>) -> Node {
	let node = node.map_children(|child| expressions(child, variables, definitions));
	let Node::List(items, Bracket::Round, Separator::None | Separator::Space) = node.drop_meta() else { return node };
	let Some(parts) = comprehension_parts(items).filter(|parts| parts.at == 1) else { return node };
	let name = symbol(&[NAME, &(definitions.len() + 1).to_string()].join(NAME_SEPARATOR));
	let free = free_variables(&parts, variables);
	let head = Node::List([name.clone()].into_iter().chain(free.iter().cloned()).collect(), Bracket::Round, Separator::None);
	definitions.push(Node::Key(Box::new(head.clone()), Op::Define, Box::new(statements(vec![yielding_loop(parts)], Bracket::Curly))));
	head.with_meta_of(&node)
}

/// The variables the expression reads besides its own loop variable
fn free_variables(parts: &Comprehension, variables: &BTreeSet<String>) -> Vec<Node> {
	let mut read = BTreeSet::new();
	for part in [Some(&parts.element), Some(&parts.sequence), parts.condition.as_ref()].into_iter().flatten() {
		part.visit(&mut |inner| read.extend(symbol_name(inner).filter(|name| variables.contains(*name)).cloned()));
	}
	read.remove(symbol_name(&parts.variable).map(String::as_str).unwrap_or_default());
	read.iter().map(|name| symbol(name)).collect()
}

fn yielding_loop(parts: Comprehension) -> Node {
	let template = if parts.condition.is_some() { FILTERED_LOOP } else { YIELDING_LOOP };
	let bindings = [("VARIABLE", Some(parts.variable)), ("SEQUENCE", Some(parts.sequence)), ("ELEMENT", Some(parts.element)), ("CONDITION", parts.condition)];
	bindings.into_iter().fold(parse(template), |node, (placeholder, value)| match value {
		Some(value) => substitute(node, placeholder, &value),
		None => node,
	})
}
