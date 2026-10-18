//! A parameter typed `data` takes its argument as an unevaluated tree (decisions.md data-parameter): `shape(code: data)
//! := code.op` called `shape(a + b)` gets `data(a + b)`, and a tag's block is the argument of a function with one data
//! parameter: `style{ ul > li { color: red } }` is `style(data({ ul > li { color: red } }))`, so a library takes a DSL
//! block. The parameter itself is `any`, the value a quoted node (lowering/control/blocks.rs). Before the quotes are
//! sealed, so no sugar pass rewrites inside an argument.
//! A name the program binds is its value in the tree (decisions.md data-names): `color: theme` splices theme's value as
//! `$(theme)` (quasi_quote.rs), an unbound word stays a word, and a part made of values only is evaluated: `"count " + n`.
use crate::lowering::blocks::{quote, DATA_WORD};
use crate::lowering::nodes::{function_head, parameter_name};
use crate::node::{children_rewritten, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const ANY_TYPE: &str = "any";
/// `$(expr)`: the splice quasi_quote.rs builds into the template
const SPLICE_CALL: &str = "$";

/// The positions of a function's parameters typed data
type DataPositions = HashMap<String, Vec<bool>>;

struct Scope<'a> {
	functions: &'a DataPositions,
	/// The names the program binds where an argument stands: its variables, functions and the enclosing parameters
	bound: HashSet<String>,
}

pub fn lower(program: Node) -> Node {
	let functions = data_functions(&program);
	if functions.is_empty() {
		return program;
	}
	// the names bound outside the data arguments: `.done = "red"` in a style sheet binds nothing
	let unspliced = quoted_arguments(program.clone(), &Scope { functions: &functions, bound: HashSet::new() });
	let scope = Scope { functions: &functions, bound: program_names(&unspliced) };
	quoted_arguments(program, &scope)
}

fn program_names(program: &Node) -> HashSet<String> {
	let mut names: HashSet<String> = program.written_names();
	program.visit(&mut |node| names.extend(function_head(node).map(|head| head.name.to_string())));
	names
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

fn quoted_arguments(node: Node, scope: &Scope) -> Node {
	if let Some(head) = function_head(&node) {
		let mut bound = scope.bound.clone();
		bound.extend(head.written_parameters.iter().filter_map(parameter_name));
		let inner = Scope { functions: scope.functions, bound };
		if scope.functions.contains_key(head.name) {
			return with_any_parameters(node, &inner);
		}
		return node.map_children(|child| quoted_arguments(child, &inner));
	}
	let node = node.map_children(|child| quoted_arguments(child, scope));
	match node {
		Node::List(items, Bracket::Round, separator) => {
			let positions = items.first().and_then(|callee| scope.functions.get(&callee.name()));
			let Some(positions) = positions else {
				return Node::List(items, Bracket::Round, separator);
			};
			let mut items = items.into_iter();
			let callee = items.next().into_iter();
			let arguments = items.enumerate().map(|(index, argument)| match positions.get(index) {
				Some(true) => quote(spliced(argument, &scope.bound)),
				_ => argument,
			});
			Node::List(callee.chain(arguments).collect(), Bracket::Round, separator)
		}
		// `style{ … }`: the block is the one data argument
		Node::Key(tag, Op::Colon, block)
			if matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _))
				&& scope.functions.get(&tag.name()).is_some_and(|positions| positions == &[true]) =>
		{
			crate::node::call(&tag.name(), vec![quote(spliced(*block, &scope.bound))])
		}
		other => other,
	}
}

/// Each largest part made of bound names and literals only, with a bound name in it, as the splice `$(part)`
fn spliced(code: Node, bound: &HashSet<String>) -> Node {
	let values_only = !code.mentions_where(|name| !bound.contains(name));
	if values_only && code.mentions_where(|name| bound.contains(name)) {
		return Node::List(vec![symbol(SPLICE_CALL), code], Bracket::Round, Separator::None);
	}
	children_rewritten(code, |child| spliced(child, bound))
}

/// The definition with its data parameters typed any, its body quoted where it calls data functions
fn with_any_parameters(definition: Node, scope: &Scope) -> Node {
	match definition {
		Node::Meta { .. } => definition.under_meta(|inner| with_any_parameters(inner, scope)),
		Node::Key(head, op, body) => {
			let head = head.under_meta(|head| match head {
				Node::List(items, bracket, separator) => {
					Node::List(items.into_iter().map(any_typed).collect(), bracket, separator)
				}
				other => other,
			});
			Node::Key(Box::new(head), op, Box::new(quoted_arguments(*body, scope)))
		}
		other => other.map_children(|child| quoted_arguments(child, scope)),
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
