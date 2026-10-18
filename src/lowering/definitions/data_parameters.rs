//! A parameter typed `data` takes its argument as an unevaluated tree (decisions.md data-parameter): `shape(code: data)
//! := code.op` called `shape(a + b)` gets `data(a + b)`, and a tag's block is the argument of a function with one data
//! parameter: `style{ ul > li { color: red } }` is `style(data({ ul > li { color: red } }))`, so a library takes a DSL
//! block. The parameter itself is `any`, the value a quoted node (lowering/control/blocks.rs). Before the quotes are
//! sealed, so no sugar pass rewrites inside an argument.
use crate::lowering::blocks::{quote, DATA_WORD};
use crate::lowering::nodes::function_head;
use crate::node::{symbol, Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;

const ANY_TYPE: &str = "any";

/// The positions of a function's parameters typed data
type DataPositions = HashMap<String, Vec<bool>>;

pub fn lower(program: Node) -> Node {
	let functions = data_functions(&program);
	if functions.is_empty() {
		return program;
	}
	quoted_arguments(program, &functions)
}

fn data_functions(program: &Node) -> DataPositions {
	let mut functions = DataPositions::new();
	program.visit(&mut |node| {
		let Some(head) = function_head(node) else {
			return;
		};
		let positions: Vec<bool> = head.written_parameters.iter().map(is_data_parameter).collect();
		if positions.contains(&true) {
			functions.insert(head.name.to_string(), positions);
		}
	});
	functions
}

/// `code: data`
fn is_data_parameter(parameter: &Node) -> bool {
	matches!(parameter.drop_meta(), Node::Key(_, Op::Colon, kind) if kind.is_symbol(DATA_WORD))
}

fn quoted_arguments(node: Node, functions: &DataPositions) -> Node {
	if let Some(head) = function_head(&node) {
		if functions.contains_key(head.name) {
			return with_any_parameters(node, functions);
		}
	}
	let node = node.map_children(|child| quoted_arguments(child, functions));
	match node {
		Node::List(items, Bracket::Round, separator) => {
			let positions = items.first().and_then(|callee| functions.get(&callee.name()));
			let Some(positions) = positions else {
				return Node::List(items, Bracket::Round, separator);
			};
			let mut items = items.into_iter();
			let callee = items.next().into_iter();
			let arguments = items.enumerate().map(|(index, argument)| match positions.get(index) {
				Some(true) => quote(argument),
				_ => argument,
			});
			Node::List(callee.chain(arguments).collect(), Bracket::Round, separator)
		}
		// `style{ … }`: the block is the one data argument
		Node::Key(tag, Op::Colon, block)
			if matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _))
				&& functions.get(&tag.name()).is_some_and(|positions| positions == &[true]) =>
		{
			crate::node::call(&tag.name(), vec![quote(*block)])
		}
		other => other,
	}
}

/// The definition with its data parameters typed any, its body quoted where it calls data functions
fn with_any_parameters(definition: Node, functions: &DataPositions) -> Node {
	match definition {
		Node::Meta { .. } => definition.under_meta(|inner| with_any_parameters(inner, functions)),
		Node::Key(head, op, body) => {
			let head = head.under_meta(|head| match head {
				Node::List(items, bracket, separator) => {
					Node::List(items.into_iter().map(any_typed).collect(), bracket, separator)
				}
				other => other,
			});
			Node::Key(Box::new(head), op, Box::new(quoted_arguments(*body, functions)))
		}
		other => other.map_children(|child| quoted_arguments(child, functions)),
	}
}

fn any_typed(parameter: Node) -> Node {
	match parameter.drop_meta() {
		Node::Key(name, Op::Colon, _) if is_data_parameter(&parameter) => {
			Node::Key(name.clone(), Op::Colon, Box::new(symbol(ANY_TYPE)))
		}
		_ => parameter,
	}
}
