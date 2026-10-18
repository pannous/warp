//! A parameter's type says what a block argument becomes. Typed `data` it takes its argument as an unevaluated tree
//! (decisions.md data-parameter): `shape(code: data) := code.op` called `shape(a + b)` gets `data(a + b)`, and a tag's
//! block is the argument of a function with one data parameter: `style{ ul > li { color: red } }` is
//! `style(data({ ul > li { color: red } }))`, so a library takes a DSL block. The parameter itself is `any`, the value a
//! quoted node (lowering/control/blocks.rs). Typed `function` a `{…}` block is the lambda of it (decision serve-plain,
//! card block-function-argument): `get "/echo" { it.body }` keeps `it => {it.body}` to call later, `run { n + 1 }`
//! gets `() => {n + 1}`; a lambda or a function passes as it is. Before the quotes are sealed, so no sugar pass rewrites
//! inside an argument.
use crate::lowering::blocks::{quote, DATA_WORD};
use crate::lowering::nodes::{function_head, is_block};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::IMPLICIT_PARAMETER;
use std::collections::HashMap;

const ANY_TYPE: &str = "any";
const FUNCTION_TYPE: &str = "function";

/// What a parameter makes of its argument
#[derive(Clone, Copy, PartialEq)]
enum Takes {
	Value,
	Data,
	Function,
}

/// A function's parameters, by what they take, of the functions with a data or function parameter
type Positions = HashMap<String, Vec<Takes>>;

pub fn lower(program: Node) -> Node {
	let functions = taking_functions(&program);
	if functions.is_empty() {
		return program;
	}
	taken_arguments(program, &functions)
}

fn taking_functions(program: &Node) -> Positions {
	let mut functions = Positions::new();
	program.visit(&mut |node| {
		let Some(head) = function_head(node) else {
			return;
		};
		let positions: Vec<Takes> = head.written_parameters.iter().map(takes).collect();
		if positions.iter().any(|taken| *taken != Takes::Value) {
			functions.insert(head.name.to_string(), positions);
		}
	});
	functions
}

/// `code: data`, `answer: function`
fn takes(parameter: &Node) -> Takes {
	match parameter.drop_meta() {
		Node::Key(_, Op::Colon, kind) if kind.is_symbol(DATA_WORD) => Takes::Data,
		Node::Key(_, Op::Colon, kind) if kind.is_symbol(FUNCTION_TYPE) => Takes::Function,
		_ => Takes::Value,
	}
}

/// The argument as the parameter takes it
fn taken(argument: Node, takes: Takes) -> Node {
	match takes {
		Takes::Data => quote(argument),
		Takes::Function if is_block(&argument) => block_lambda(argument),
		_ => argument,
	}
}

/// `{ it * 2 }` → `it => {it * 2}`, `{ n + 1 }` → `() => {n + 1}`
fn block_lambda(block: Node) -> Node {
	let parameters = match block.mentions(IMPLICIT_PARAMETER) {
		true => symbol(IMPLICIT_PARAMETER),
		false => Node::List(vec![], Bracket::Round, Separator::None),
	};
	Node::Key(Box::new(parameters), Op::Arrow, Box::new(block))
}

fn taken_arguments(node: Node, functions: &Positions) -> Node {
	if let Some(head) = function_head(&node) {
		if functions.contains_key(head.name) {
			return with_any_parameters(node, functions);
		}
	}
	let node = node.map_children(|child| taken_arguments(child, functions));
	match node {
		Node::List(items, bracket @ Bracket::Round, separator) => with_taken(items, bracket, separator, functions, |_| true),
		// `get "/echo" { it.body }`: a spaced call's blocks
		Node::List(items, bracket @ Bracket::None, separator @ Separator::Space) => {
			with_taken(items, bracket, separator, functions, is_block)
		}
		// `style{ … }`, `run{ … }`: the block is the one argument
		Node::Key(tag, Op::Colon, block)
			if is_block(&block)
				&& functions.get(&tag.name()).is_some_and(|positions| positions.len() == 1 && positions[0] != Takes::Value) =>
		{
			crate::node::call(&tag.name(), vec![taken(*block, functions[&tag.name()][0])])
		}
		other => other,
	}
}

/// The call `items` with each argument as its parameter takes it, of those `changes` picks
fn with_taken(
	items: Vec<Node>,
	bracket: Bracket,
	separator: Separator,
	functions: &Positions,
	changes: fn(&Node) -> bool,
) -> Node {
	let Some(positions) = items.first().and_then(|callee| functions.get(&callee.name())) else {
		return Node::List(items, bracket, separator);
	};
	let mut items = items.into_iter();
	let callee = items.next().into_iter();
	let arguments = items.enumerate().map(|(index, argument)| match positions.get(index) {
		Some(takes) if changes(&argument) => taken(argument, *takes),
		_ => argument,
	});
	Node::List(callee.chain(arguments).collect(), bracket, separator)
}

/// The definition with its data parameters typed any, its body's arguments taken as their parameters take them
fn with_any_parameters(definition: Node, functions: &Positions) -> Node {
	match definition {
		Node::Meta { .. } => definition.under_meta(|inner| with_any_parameters(inner, functions)),
		Node::Key(head, op, body) => {
			let head = head.under_meta(|head| match head {
				Node::List(items, bracket, separator) => {
					Node::List(items.into_iter().map(any_typed).collect(), bracket, separator)
				}
				other => other,
			});
			Node::Key(Box::new(head), op, Box::new(taken_arguments(*body, functions)))
		}
		other => other.map_children(|child| taken_arguments(child, functions)),
	}
}

fn any_typed(parameter: Node) -> Node {
	match takes(&parameter) {
		Takes::Data => {
			let Node::Key(name, _, _) = parameter.drop_meta() else { unreachable!("a typed parameter") };
			Node::Key(name.clone(), Op::Colon, Box::new(symbol(ANY_TYPE)))
		}
		_ => parameter,
	}
}
