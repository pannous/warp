//! Functions broadcast over lists and pairs (wiki/broadcasting.md, notes/broadcasting.md): `square [1 2 3]` is
//! `[square(1) square(2) square(3)]`, `square [a:1 b:2]` is `[a:square(1) b:square(2)]`, and a variable only ever assigned
//! a list literal is mapped (`square xs` → `map xs square`). Only a function of one undeclared parameter that the body
//! uses as an arithmetic operand broadcasts; a function of a list (`count(xs)`, `xs#2`) takes the list whole. A declared
//! parameter (`x:int`, `square number`) keeps its type error (open decision P50). Operators never broadcast (`[1 2 3]*2`).

use crate::function_values::{definitions, Definition};
use crate::lambdas::IMPLICIT_PARAMETER;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const ARITHMETIC: [Op; 6] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Pow];
const MAP_WORD: &str = "map";

pub fn lower(program: Node) -> Node {
	let mut found = Vec::new();
	definitions(&program, &mut found);
	implicit_definitions(&program, &mut found);
	let broadcasting: HashSet<String> = found.iter().filter(|definition| needs_a_scalar(definition)).map(|definition| definition.name.clone()).collect();
	if broadcasting.is_empty() {
		return program;
	}
	let mut assigned = HashMap::new();
	collect_list_variables(&program, &mut assigned);
	let list_variables = assigned.into_iter().filter(|(_, only_lists)| *only_lists).map(|(name, _)| name).collect();
	Broadcast { functions: broadcasting, list_variables }.rewrite(program)
}

/// `square := it*it`: a function of the implicit parameter `it`
fn implicit_definitions(node: &Node, found: &mut Vec<Definition>) {
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Define, body) = part {
			if let Node::Symbol(name) = target.drop_meta() {
				let params = vec![Node::Symbol(IMPLICIT_PARAMETER.to_string())];
				found.push(Definition { name: name.clone(), params, body: body.as_ref().clone() });
			}
		}
	});
}

fn needs_a_scalar(definition: &Definition) -> bool {
	let [param] = definition.params.as_slice() else { return false };
	matches!(param.drop_meta(), Node::Symbol(name) if is_arithmetic_operand(&definition.body, name))
}

fn is_arithmetic_operand(body: &Node, name: &str) -> bool {
	let is_param = |node: &Node| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name);
	let mut found = false;
	body.visit(&mut |node| {
		if let Node::Key(left, op, right) = node {
			found |= ARITHMETIC.contains(op) && (is_param(left) || is_param(right));
		}
	});
	found
}

/// Variable → whether every value assigned to it is a list literal that is no object
fn collect_list_variables(node: &Node, assigned: &mut HashMap<String, bool>) {
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if let Node::Symbol(name) = target.drop_meta() {
				let is_list = matches!(value.drop_meta(), Node::List(items, Bracket::Square, _) if !items.iter().any(is_pair));
				*assigned.entry(name.clone()).or_insert(true) &= is_list;
			}
		}
	});
}

fn is_pair(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Colon, _))
}

fn call(name: &str, argument: Node) -> Node {
	Node::List(vec![Node::Symbol(name.to_string()), argument], Bracket::Round, Separator::None)
}

struct Broadcast {
	functions: HashSet<String>,
	list_variables: HashSet<String>,
}

impl Broadcast {
	fn rewrite(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.rewrite(item)).collect();
				self.broadcast_call(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}

	/// `f [1 2]`, `f([1 2])`, `f xs` of a broadcasting function `f`
	fn broadcast_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let is_call = matches!((bracket, separator), (Bracket::Round, Separator::None) | (Bracket::None, Separator::Space));
		let [head, argument] = items else { return None };
		let Node::Symbol(name) = head.drop_meta() else { return None };
		if !is_call || !self.functions.contains(name) {
			return None;
		}
		match argument.drop_meta() {
			Node::List(elements, Bracket::Square, element_separator) if !elements.is_empty() => {
				let applied = elements.iter().map(|element| self.apply(name, element.clone())).collect();
				Some(Node::List(applied, Bracket::Square, element_separator.clone()))
			}
			Node::Symbol(variable) if self.list_variables.contains(variable) => {
				Some(Node::List(vec![Node::Symbol(MAP_WORD.to_string()), argument.clone(), head.clone()], Bracket::Round, Separator::None))
			}
			_ => None,
		}
	}

	/// The function applied to one element: to the value of a pair, broadcast again over a nested list
	fn apply(&self, name: &str, element: Node) -> Node {
		match element.drop_meta() {
			Node::Key(key, Op::Colon, value) => Node::Key(key.clone(), Op::Colon, Box::new(self.apply(name, value.as_ref().clone()))),
			_ => {
				let items = vec![Node::Symbol(name.to_string()), element];
				self.broadcast_call(&items, &Bracket::Round, &Separator::None).unwrap_or_else(|| call(name, items[1].clone()))
			}
		}
	}
}
