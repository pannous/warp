//! Generator expressions (card generators-generator): `(x * x for x in xs)` and `(x for x in xs if x > 1)` are
//! generators. Each becomes the generator `generator·expression·1(free…) := { for x in xs { if c { yield x * x } } }`
//! and its call, so a loop or take over it runs lazily, `next` resumes it and any other use collects (generators.rs).
//! Its parameters are the variables it reads, passed when it is made. An element of several words is no generator
//! expression: `(upper w for w in words)` reads as the call `upper(w for w in words)` (comprehensions.rs).

use crate::comprehensions::{bound_names, Comprehension};
use crate::generators::NAME_SEPARATOR;
use super::words::FOR_WORD;
use super::nodes::{key, statement_list, symbol};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::BTreeSet;

const NAME: &str = "generator·expression";

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
	statement_list(definitions.into_iter().chain(items).collect(), Bracket::None)
}

/// The names the program assigns, takes as parameters or loops over
fn bound_variables(node: &Node) -> BTreeSet<String> {
	let mut variables = BTreeSet::new();
	node.visit(&mut |part| match part {
		Node::Key(target, Op::Assign | Op::Define, _) => variables.extend(bound_names(target)),
		Node::List(words, _, _) if words.first().is_some_and(|word| word.symbol_name().is_some_and(|name| name == FOR_WORD)) => {
			variables.extend(words.get(1).and_then(Node::symbol_name).map(String::from));
		}
		_ => {}
	});
	variables
}

fn expressions(node: Node, variables: &BTreeSet<String>, definitions: &mut Vec<Node>) -> Node {
	let node = node.map_children(|child| expressions(child, variables, definitions));
	let Node::List(items, Bracket::Round, Separator::None | Separator::Space) = node.drop_meta() else { return node };
	let Some(comprehension) = Comprehension::of(items).filter(|comprehension| comprehension.at == 1) else { return node };
	let name = symbol(&[NAME, &(definitions.len() + 1).to_string()].join(NAME_SEPARATOR));
	let free = free_variables(&comprehension, variables);
	let head = Node::List([name.clone()].into_iter().chain(free.iter().cloned()).collect(), Bracket::Round, Separator::None);
	definitions.push(key(head.clone(), Op::Define, statement_list(vec![comprehension.yielding_loop()], Bracket::Curly)));
	head.with_meta_of(&node)
}

/// The variables the expression reads besides its own loop variables
fn free_variables(comprehension: &Comprehension, variables: &BTreeSet<String>) -> Vec<Node> {
	let mut read = BTreeSet::new();
	for part in comprehension.parts() {
		part.visit(&mut |inner| read.extend(inner.symbol_name().filter(|name| variables.contains(*name)).map(String::from)));
	}
	for own in comprehension.loop_variables() {
		read.remove(&own);
	}
	read.iter().map(|name| symbol(name)).collect()
}
