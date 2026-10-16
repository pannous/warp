//! A bare name of a function without parameters is its call (decision reflection, P82), also where its value is used:
//! `x = a` holds what a returns, `a + 1` adds to it (card assigning-bare). `function a` stays the function, a variable
//! or parameter of that name stays itself. A word of the host taking nothing, `random`, is called the same way, and also
//! where the emitter calls a program's function by itself: as a statement, after `print` and `type` (card random-should).
//! A name overloaded by arity with a variant taking nothing, `canvas()` beside `canvas(width, height)`, calls that
//! variant `canvas·0`, also as a call's argument: `save(canvas, "x.png")` (decision saved-words).

use super::nodes::{call, children_rewritten};
use crate::lowering::reflection::CALLING_WORDS;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

/// A bare name and the function it calls
type Calls = HashMap<String, String>;

/// The names a bare word calls: every function without parameters, of them the host's, and the arity overloads'
struct Parameterless {
	all: Calls,
	host: Calls,
	overloaded: Calls,
}

pub fn lower(node: Node) -> Node {
	let parameterless = parameterless_functions(&node);
	if parameterless.all.is_empty() {
		return node;
	}
	let node = rewrite(node, &parameterless);
	called_value(node, &parameterless.host)
}

/// The functions without parameters, the program's and the host's, whose names no variable or parameter takes
fn parameterless_functions(node: &Node) -> Parameterless {
	let functions = crate::analyzer::function_definitions(node).user_functions;
	let assigned: HashSet<String> = node.assigned_names(|op| *op == Op::Assign);
	let parameters: HashSet<&str> =
		functions.values().flat_map(|function| function.params.iter().map(|param| param.name.as_str())).collect();
	let free = |name: &str| !assigned.contains(name) && !parameters.contains(name);
	let host: Calls = crate::host::parameterless_host_words()
		.filter(|name| free(name) && !functions.contains_key(*name))
		.map(|name| (name.to_string(), name.to_string()))
		.collect();
	let parameterless = functions.iter().filter(|(_, function)| function.params.is_empty());
	let overloaded: Calls = parameterless
		.clone()
		.filter_map(|(name, _)| zero_arity_base(name).map(|base| (base.to_string(), name.clone())))
		.filter(|(base, _)| free(base) && !functions.contains_key(base))
		.collect();
	let program = parameterless.filter(|(name, _)| free(name)).map(|(name, _)| (name.clone(), name.clone()));
	let all = program.chain(host.clone()).chain(overloaded.clone()).collect();
	Parameterless { all, host, overloaded }
}

/// `canvas` of `canvas·0`, the variant of an arity overload taking nothing
fn zero_arity_base(name: &str) -> Option<&str> {
	name.strip_suffix(&format!("{}0", crate::traits::WITNESS_SEPARATOR))
}

fn rewrite(node: Node, parameterless: &Parameterless) -> Node {
	let called = |side: Node| called_value(side, &parameterless.all);
	let host_called = |side: Node| called_value(side, &parameterless.host);
	match children_rewritten(node, |child| rewrite(child, parameterless)) {
		Node::Key(target, Op::Assign, value) => Node::Key(target, Op::Assign, Box::new(called(*value))),
		Node::Key(left, op, right) if op.is_operation() => {
			Node::Key(Box::new(called(*left)), op, Box::new(called(*right)))
		}
		// `random as int`: the value cast, never the type
		Node::Key(value, Op::As, type_name) => Node::Key(Box::new(called(*value)), Op::As, type_name),
		Node::List(items, bracket, separator) if separator.separates_statements() => {
			Node::List(items.into_iter().map(host_called).collect(), bracket, separator)
		}
		Node::List(items, bracket @ (Bracket::Round | Bracket::None), separator) if matches!(items.as_slice(), [word, _] if CALLING_WORDS.iter().any(|calling| word.is_symbol(calling))) =>
		{
			let mut items = items;
			let argument = host_called(items.pop().expect("the argument"));
			items.push(argument);
			Node::List(items, bracket, separator)
		}
		// `save(canvas, "x.png")`: an overloaded name is no one function value, so as an argument it is its call
		Node::List(items, Bracket::Round, separator)
			if items.first().is_some_and(|head| head.symbol_name().is_some()) =>
		{
			let mut items = items.into_iter();
			let head = items.next().expect("the called name");
			let arguments = items.map(|argument| called_value(argument, &parameterless.overloaded));
			Node::List(std::iter::once(head).chain(arguments).collect(), Bracket::Round, separator)
		}
		other => other,
	}
}

/// `a()` of the bare name a, written without `function`
fn called_value(value: Node, calls: &Calls) -> Node {
	let called = value.drop_meta().symbol_name().and_then(|name| calls.get(name)).cloned();
	match called {
		Some(function) if crate::closures::referenced_function(&value).is_none() => call(&function, vec![]),
		_ => value,
	}
}
