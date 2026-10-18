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
use crate::lowering::nodes::{for_in_variable, function_head, is_block, parameter_name};
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
	// the names bound outside the data arguments: `.done = "red"` in a style sheet binds nothing
	let unspliced = taken_arguments(program.clone(), &Scope { functions: &functions, bound: HashSet::new() });
	let scope = Scope { functions: &functions, bound: program_names(&unspliced) };
	taken_arguments(program, &scope)
}

/// The program's variables, its functions and its loop variables (`[li{ todo.text } for todo in todos]`)
fn program_names(program: &Node) -> HashSet<String> {
	let mut names: HashSet<String> = program.written_names();
	program.visit(&mut |node| {
		names.extend(function_head(node).map(|head| head.name.to_string()));
		if let Node::List(items, _, _) = node {
			names.extend((0..items.len()).filter_map(|start| for_in_variable(&items[start..])));
		}
	});
	names
}

/// The program's functions taking data or a function, and the standard ones taking data it does not define
fn taking_functions(program: &Node) -> Positions {
	let mut functions = Positions::new();
	functions.extend(std_functions_taking());
	// a function of the program's own wins
	program.visit(&mut |node| {
		if let Some(head) = function_head(node) {
			functions.remove(head.name);
		}
	});
	functions.extend(functions_taking(program));
	functions
}

/// The standard functions taking data, their definitions parsed once per thread and quietly: no hint about library code
fn std_functions_taking() -> Positions {
	thread_local!(static FUNCTIONS: Positions = crate::normalize::without_hints(|| {
		let functions = crate::modules::std_data_functions().iter();
		// the head alone: a body's `{` opening on the line would leave its missing-`}` fix in the program's report
		let head = |line: &str| format!("{} := ø", line.split(":=").next().unwrap_or_default());
		functions.flat_map(|(_, line)| functions_taking(&crate::warp_parser::parse(&head(line)))).collect()
	}));
	FUNCTIONS.with(Clone::clone)
}

fn functions_taking(program: &Node) -> Positions {
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
		Takes::Function if is_block(&argument) => block_lambda(taken_arguments(argument, scope)),
		_ => taken_arguments(argument, scope),
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
	match node {
		Node::List(items, bracket @ Bracket::Round, separator) => {
			with_taken(items, bracket, separator, scope, |_| true)
		}
		// `get "/echo" { it.body }`: a spaced call's blocks
		Node::List(items, bracket @ Bracket::None, separator @ Separator::Space) => {
			with_taken(items, bracket, separator, scope, is_block)
		}
		// `style{ … }`, `run{ … }`: the block is the one argument, taken as written before any part of it is
		Node::Key(tag, Op::Colon, block) if is_block(&block) => match one_taking(&tag, scope) {
			Some(takes) => crate::node::call(&tag.name(), vec![taken(*block, takes, scope)]),
			None => Node::Key(tag, Op::Colon, Box::new(tag_block(*block, scope))),
		},
		other => other.map_children(|child| taken_arguments(child, scope)),
	}
}

/// What the one parameter of the function `tag` names takes, of a function with one data or function parameter
fn one_taking(tag: &Node, scope: &Scope) -> Option<Takes> {
	match scope.functions.get(&tag.symbol_name()?.to_string()).map(Vec::as_slice) {
		Some(&[takes]) if takes != Takes::Value => Some(takes),
		_ => None,
	}
}

/// A tag's block: in it `transition: opacity 200ms ease-out` is the call of a function with one data parameter, its
/// words the bare words and quantities after the colon, up to the children (decisions.md transitions-data)
fn tag_block(block: Node, scope: &Scope) -> Node {
	block.under_meta(|block| match block {
		Node::List(items, bracket, separator) => Node::List(entries_taken(items, scope), bracket, separator),
		other => taken_arguments(other, scope),
	})
}

fn entries_taken(items: Vec<Node>, scope: &Scope) -> Vec<Node> {
	let mut taken_items = vec![];
	let mut items = items.into_iter().flat_map(line_items).peekable();
	while let Some(item) = items.next() {
		let Some((tag, value)) = data_key(&item, scope) else {
			taken_items.push(taken_arguments(item, scope));
			continue;
		};
		let mut words = vec![value];
		while let Some(word) = items.next_if(|next| is_bare_word(next, scope)) {
			words.push(word);
		}
		let body = if words.len() == 1 { words.remove(0) } else { Node::List(words, Bracket::None, Separator::Space) };
		taken_items.push(crate::node::call(&tag, vec![taken(body, Takes::Data, scope)]));
	}
	taken_items
}

/// The items of a line of words `transition: fade 200ms "a"` one by one, any other item as it is
fn line_items(item: Node) -> Vec<Node> {
	match item.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space)
			if items.first().is_some_and(|first| matches!(first.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			items.clone()
		}
		_ => vec![item],
	}
}

/// `transition: fade`: the function's name and the value after its colon
fn data_key(item: &Node, scope: &Scope) -> Option<(String, Node)> {
	match item.drop_meta() {
		Node::Key(tag, Op::Colon, value) if one_taking(tag, scope) == Some(Takes::Data) => {
			Some((tag.name(), value.as_ref().clone()))
		}
		_ => None,
	}
}

/// `ease-out`, `400ms`, `2`: a word of a data entry, no name the program binds
fn is_bare_word(node: &Node, scope: &Scope) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => !scope.bound.contains(name),
		Node::Number(_) => true,
		Node::Key(amount, Op::Mul, unit) => {
			matches!(amount.drop_meta(), Node::Number(_)) && matches!(unit.drop_meta(), Node::Symbol(_))
		}
		_ => false,
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
	let positions = items.first().and_then(|callee| scope.functions.get(&callee.name()));
	let arguments = items.into_iter().enumerate().map(|(index, item)| {
		match positions.and_then(|positions| positions.get(index.checked_sub(1)?)) {
			Some(takes) if changes(&item) => taken(item, *takes, scope),
			_ => taken_arguments(item, scope),
		}
	});
	Node::List(arguments.collect(), bracket, separator)
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
