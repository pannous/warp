//! A parameter's type says what a block argument becomes. Typed `data` it takes its argument as an unevaluated tree
//! (decisions.md data-parameter): `shape(code: data) := code.op` called `shape(a + b)` gets `data(a + b)`, and a tag's
//! block is the argument of a function with one data parameter: `style{ ul > li { color: red } }` is
//! `style(data({ ul > li { color: red } }))`, so a library takes a DSL block. The parameter itself is `any`, the value a
//! quoted node (lowering/control/blocks.rs). A name the program binds is its value in the tree (decisions.md
//! data-names): `color: theme` splices theme's value as `$(theme)` (quasi_quote.rs), an unbound word stays a word, and a
//! part made of values only is evaluated: `"count " + n`.
//! Typed `function` a `{…}` block is the lambda of it (decision serve-plain, card block-function-argument):
//! `get "/echo" { it.body }` keeps `it => {it.body}` to call later, `run { n + 1 }` gets `() => {n + 1}`; a lambda or a
//! function passes as it is. Before the quotes are sealed, so no sugar pass rewrites inside an argument.
use crate::lowering::blocks::{quote, DATA_WORD};
use crate::lowering::nodes::{function_head, is_block, parameter_name};
use crate::node::{children_rewritten, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::IMPLICIT_PARAMETER;
use std::collections::{HashMap, HashSet};

const ANY_TYPE: &str = "any";
const FUNCTION_TYPE: &str = "function";
/// `$(expr)`: the splice quasi_quote.rs builds into the template
const SPLICE_CALL: &str = "$";

/// What a parameter makes of its argument
#[derive(Clone, Copy, PartialEq)]
enum Takes {
	Value,
	Data,
	Function,
}

/// A function's parameters, by what they take, of the functions with a data or function parameter
type Positions = HashMap<String, Vec<Takes>>;

struct Scope<'a> {
	functions: &'a Positions,
	/// The names the program binds where an argument stands: its variables, functions and the enclosing parameters
	bound: HashSet<String>,
}

pub fn lower(program: Node) -> Node {
	let functions = taking_functions(&program);
	if functions.is_empty() {
		return program;
	}
	let scope = Scope { functions: &functions, bound: program_names(&program) };
	taken_arguments(program, &scope)
}

fn program_names(program: &Node) -> HashSet<String> {
	let mut names: HashSet<String> = program.written_names();
	program.visit(&mut |node| names.extend(function_head(node).map(|head| head.name.to_string())));
	names
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
		Node::Key(_, Op::Colon, kind) if is_function_type(kind) => Takes::Function,
		_ => Takes::Value,
	}
}

/// `function` or its alias `closure`
pub(crate) fn is_function_type(kind: &Node) -> bool {
	kind.symbol_name().and_then(crate::type_tests::builtin_type_name) == Some(FUNCTION_TYPE)
}

/// The argument as the parameter takes it
fn taken(argument: Node, takes: Takes, scope: &Scope) -> Node {
	match takes {
		Takes::Data => quote(spliced(argument, &scope.bound)),
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

/// Each largest part made of bound names and literals only, with a bound name in it, as the splice `$(part)`
fn spliced(code: Node, bound: &HashSet<String>) -> Node {
	let values_only = !code.mentions_where(|name| !bound.contains(name));
	if values_only && code.mentions_where(|name| bound.contains(name)) {
		return Node::List(vec![symbol(SPLICE_CALL), code], Bracket::Round, Separator::None);
	}
	children_rewritten(code, |child| spliced(child, bound))
}

fn taken_arguments(node: Node, scope: &Scope) -> Node {
	if let Some(head) = function_head(&node) {
		let mut bound = scope.bound.clone();
		bound.extend(head.written_parameters.iter().filter_map(parameter_name));
		let inner = Scope { functions: scope.functions, bound };
		if scope.functions.contains_key(head.name) {
			return with_any_parameters(node, &inner);
		}
		return node.map_children(|child| taken_arguments(child, &inner));
	}
	let node = node.map_children(|child| taken_arguments(child, scope));
	match node {
		Node::List(items, bracket @ Bracket::Round, separator) => {
			with_taken(items, bracket, separator, scope, |_| true)
		}
		// `get "/echo" { it.body }`: a spaced call's blocks
		Node::List(items, bracket @ Bracket::None, separator @ Separator::Space) => {
			with_taken(items, bracket, separator, scope, is_block)
		}
		// `style{ … }`, `run{ … }`: the block is the one argument
		Node::Key(tag, Op::Colon, block) if is_block(&block) => {
			match scope.functions.get(&tag.name()).map(Vec::as_slice) {
				Some(&[takes]) if takes != Takes::Value => {
					crate::node::call(&tag.name(), vec![taken(*block, takes, scope)])
				}
				_ => Node::Key(tag, Op::Colon, block),
			}
		}
		other => other,
	}
}

/// The call `items` with each argument as its parameter takes it, of those `changes` picks
fn with_taken(
	items: Vec<Node>,
	bracket: Bracket,
	separator: Separator,
	scope: &Scope,
	changes: fn(&Node) -> bool,
) -> Node {
	let Some(positions) = items.first().and_then(|callee| scope.functions.get(&callee.name())) else {
		return Node::List(items, bracket, separator);
	};
	let mut items = items.into_iter();
	let callee = items.next().into_iter();
	let arguments = items.enumerate().map(|(index, argument)| match positions.get(index) {
		Some(takes) if changes(&argument) => taken(argument, *takes, scope),
		_ => argument,
	});
	Node::List(callee.chain(arguments).collect(), bracket, separator)
}

/// The definition with its data parameters typed any, its body's arguments taken as their parameters take them
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
			Node::Key(Box::new(head), op, Box::new(taken_arguments(*body, scope)))
		}
		other => other.map_children(|child| taken_arguments(child, scope)),
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
