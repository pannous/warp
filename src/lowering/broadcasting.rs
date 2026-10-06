//! Functions broadcast over lists and pairs (wiki/broadcasting.md, notes/broadcasting.md): `square [1 2 3]` is
//! `[square(1) square(2) square(3)]`, `square [a:1 b:2]` is `[a:square(1) b:square(2)]`, and a variable only ever assigned
//! a list literal is mapped (`square xs` → `map xs square`). A function of one parameter broadcasts when the parameter is
//! declared with a scalar type (`x:int`, `square number`, P50) or, undeclared, is an arithmetic operand of the body; a
//! function of a list (`count(xs)`, `xs#2`, `xs:list`) takes the list whole. Operators never broadcast (`[1 2 3]*2`).

use crate::analyzer::annotated_kind;
use crate::function_values::{definitions, Definition};
use crate::type_kinds::Kind;
use crate::lambdas::IMPLICIT_PARAMETER;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const ARITHMETIC: [Op; 6] = [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::Mod, Op::Pow];
const MAP_WORD: &str = "map";
/// Declared parameter kinds that take one element of a list (P50)
const SCALAR_KINDS: [Kind; 4] = [Kind::Int, Kind::Float, Kind::Text, Kind::Codepoint];

/// P84 (user: "This should have already been done with broadcasting"): several juxtaposed arguments of a function of one
/// parameter are one list, `sum 1 2 3` is `sum [1 2 3]`, and a scalar function broadcasts over it (`square 1 2 3`).
/// After the lambda passes, which make `sum := fold +` a function of one parameter; `f(1, 2, 3)` stays an arity error.
pub fn lower_several_arguments(program: Node) -> Node {
	let mut found = Vec::new();
	definitions(&program, &mut found);
	implicit_definitions(&program, &mut found);
	let single: HashSet<String> = found.iter().filter(|definition| definition.params.len() == 1).map(|definition| definition.name.clone()).collect();
	if single.is_empty() {
		return program;
	}
	let mut gathered = false;
	let program = gather_arguments(program, &single, &mut gathered);
	if gathered { lower(program) } else { program }
}

/// Functions are prefix operators (wiki/function.md): when a call has more juxtaposed arguments than its function takes,
/// the last function name among them takes the arguments after it, `square square 2` → `square (square 2)`. Before the
/// function value passes read that name as a value
pub fn lower_prefix_calls(program: Node) -> Node {
	let mut found = Vec::new();
	definitions(&program, &mut found);
	let mut arities: HashMap<String, usize> = HashMap::new();
	for definition in &found {
		let most = arities.entry(definition.name.clone()).or_default();
		*most = (*most).max(definition.params.len());
	}
	if arities.is_empty() { program } else { nest_prefix_calls(program, &arities) }
}

fn nest_prefix_calls(node: Node, arities: &HashMap<String, usize>) -> Node {
	let arity = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) => arities.get(name).copied(),
		_ => None,
	};
	match node {
		Node::List(items, bracket @ (Bracket::None | Bracket::Round), separator @ (Separator::Space | Separator::None))
			if items.len() > 2 && arity(&items[0]).is_some_and(|takes| items.len() - 1 > takes) =>
		{
			let mut items: Vec<Node> = items.into_iter().map(|item| nest_prefix_calls(item, arities)).collect();
			let Some(inner) = (1..items.len() - 1).rev().find(|index| arity(&items[*index]).is_some()) else {
				return Node::List(items, bracket, separator);
			};
			let nested = Node::List(items.split_off(inner), Bracket::None, Separator::Space);
			items.push(nest_prefix_calls(nested, arities));
			nest_prefix_calls(Node::List(items, bracket, separator), arities)
		}
		other => other.map_children(|child| nest_prefix_calls(child, arities)),
	}
}

/// `f 1 2 3` → `f [1 2 3]` of the functions `single`
fn gather_arguments(node: Node, single: &HashSet<String>, gathered: &mut bool) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space) if items.len() > 2 && matches!(items[0].drop_meta(), Node::Symbol(name) if single.contains(name)) => {
			*gathered = true;
			let mut items: Vec<Node> = items.into_iter().map(|item| gather_arguments(item, single, gathered)).collect();
			let arguments = items.split_off(1);
			items.push(Node::List(arguments, Bracket::Square, Separator::Space));
			Node::List(items, Bracket::None, Separator::Space)
		}
		other => other.map_children(|child| gather_arguments(child, single, gathered)),
	}
}

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
	match param.drop_meta() {
		Node::Key(_, Op::Colon, type_node) => annotated_kind(type_node).is_some_and(|kind| SCALAR_KINDS.contains(&kind)),
		Node::Symbol(name) => is_arithmetic_operand(&definition.body, name),
		_ => false,
	}
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
				// `map square [1 2 3]`: the iteration word applies square itself (lambdas.rs), no broadcast inside it
				let iterates = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if crate::function_values::ITERATION_WORDS.contains(&word.as_str()));
				let items: Vec<Node> = items.into_iter().map(|item| match item.drop_meta() {
					Node::List(inner, inner_bracket, inner_separator) if iterates => {
						Node::List(inner.iter().cloned().map(|part| self.rewrite(part)).collect(), inner_bracket.clone(), inner_separator.clone())
					}
					_ => self.rewrite(item),
				}).collect();
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
