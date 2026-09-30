//! Compile-time lambdas. Functions are not first-class values yet, so a lambda is lowered where it stands:
//! - `f = x=>x*x`, `f = (x y)->x+y` define the function `f` at that point (captured variables by value, like every
//!   definition); `f 3` calls it; the parser itself reads `f = {it*2}` as the definition `f := {it*2}`
//! - `{x*x}(x=5)` defines an anonymous function and calls it at once
//! - `map [1 2 3] {it*it}`, `map(xs, x=>x+1)`, `xs.map(f)` over a literal block, a lambda or a defined function is a loop
//! - a lambda anywhere else (an argument of another call, `map` over a non-function) is the error `functions are not first-class values yet`

use crate::analyzer::{call_name, extract_user_functions};
use crate::context::Context;
use crate::diagnostic::Diagnostic;
use crate::library_words::substitute;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::parse;
use std::cell::Cell;

pub const NOT_FIRST_CLASS: &str = "functions are not first-class values yet";
const IMPLICIT_PARAMETER: &str = "it";
const LIST_PLACEHOLDER: &str = "loop_list";
const START_PLACEHOLDER: &str = "loop_start";
const CALL_PLACEHOLDER: &str = "loop_call";

/// An iteration word over a list: the arguments between the list and the function, the arguments of the function
/// (`reduce` and `fold` take the accumulator and the item), and the loop it lowers to. `out`, `item`, `acc`, `list`, `index`
/// are replaced by fresh names.
struct Iteration {
	word: &'static str,
	extra_arguments: usize,
	function_arguments: usize,
	template: &'static str,
}

const ITERATIONS: [Iteration; 5] = [
	Iteration { word: "map", extra_arguments: 0, function_arguments: 1, template: "(out=[]; for item in loop_list { out.add(loop_call) }; out)" },
	Iteration { word: "filter", extra_arguments: 0, function_arguments: 1, template: "(out=[]; for item in loop_list { if loop_call { out.add(item) } }; out)" },
	Iteration { word: "each", extra_arguments: 0, function_arguments: 1, template: "(value=0; for item in loop_list { value = loop_call }; value)" },
	Iteration { word: "fold", extra_arguments: 1, function_arguments: 2, template: "(acc=loop_start; for item in loop_list { acc = loop_call }; acc)" },
	Iteration {
		word: "reduce",
		extra_arguments: 0,
		function_arguments: 2,
		template: "(list=loop_list; if count(list) == 0 then empty_extremum(reduce) else (acc=list#1; index=2; while index <= count(list) { item=list#index; acc = loop_call; index=index+1 }; acc))",
	},
];

struct Lambda {
	params: Vec<String>,
	body: Node,
}

pub fn lower(node: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	Lowering { context, counter: Cell::new(0) }.expand(node)
}

fn mentions(node: &Node, name: &str) -> bool {
	match node.drop_meta() {
		Node::Symbol(symbol) => symbol == name,
		Node::Key(left, _, right) => mentions(left, name) || mentions(right, name),
		Node::List(items, _, _) => items.iter().any(|item| mentions(item, name)),
		_ => false,
	}
}

/// `a-b` in the body of `(a b)->a-b` reads as one hyphenated name: it is the difference when its parts are parameters
fn subtract_kebab_parameters(node: Node, params: &[String]) -> Node {
	match node {
		Node::Symbol(name) => {
			let parts: Vec<&str> = name.split('-').collect();
			if parts.len() > 1 && parts.iter().all(|part| params.iter().any(|param| param == part)) {
				let mut terms = parts.into_iter().map(|part| Node::Symbol(part.to_string()));
				let first = terms.next().expect("split gives a part");
				terms.fold(first, |difference, term| Node::Key(Box::new(difference), Op::Sub, Box::new(term)))
			} else {
				Node::Symbol(name)
			}
		}
		Node::Key(left, op, right) => Node::Key(Box::new(subtract_kebab_parameters(*left, params)), op, Box::new(subtract_kebab_parameters(*right, params))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| subtract_kebab_parameters(item, params)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(subtract_kebab_parameters(*node, params)), data },
		other => other,
	}
}

fn parameter_names(left: &Node) -> Option<Vec<String>> {
	match left.drop_meta() {
		Node::Symbol(name) => Some(vec![name.clone()]),
		Node::List(items, _, _) => items
			.iter()
			.map(|item| match item.drop_meta() {
				Node::Symbol(name) => Some(name.clone()),
				_ => None,
			})
			.collect(),
		_ => None,
	}
}

/// `x=>body`, `(x y)->body` and the same in a group `(x=>body)`
fn arrow_lambda(node: &Node) -> Option<Lambda> {
	match node.drop_meta() {
		Node::Key(left, Op::Arrow | Op::FatArrow, body) => {
			let params = parameter_names(left)?;
			let body = subtract_kebab_parameters(body.as_ref().clone(), &params);
			Some(Lambda { params, body })
		}
		Node::List(items, Bracket::Round, _) if items.len() == 1 => arrow_lambda(&items[0]),
		_ => None,
	}
}

/// `{it*it}`: a block that is no object; its parameter is `it` when the body uses it
fn block_lambda(node: &Node) -> Option<Lambda> {
	let Node::List(items, Bracket::Curly, separator) = node.drop_meta() else { return None };
	if items.is_empty() || items.iter().any(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, _))) {
		return None;
	}
	let body = match items.as_slice() {
		[single] => single.clone(),
		many => Node::List(many.to_vec(), Bracket::Round, separator.clone()),
	};
	let params = if mentions(&body, IMPLICIT_PARAMETER) { vec![IMPLICIT_PARAMETER.to_string()] } else { vec![] };
	Some(Lambda { params, body })
}

fn definition(name: &str, lambda: Lambda) -> Node {
	let head = Node::List([vec![Node::Symbol(name.to_string())], lambda.params.into_iter().map(Node::Symbol).collect()].concat(), Bracket::Round, Separator::None);
	Node::Key(Box::new(head), Op::Define, Box::new(lambda.body))
}

fn call(name: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(name.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}

struct Lowering {
	context: Context,
	counter: Cell<usize>,
}

impl Lowering {
	fn fresh(&self, prefix: &str) -> String {
		let number = self.counter.get();
		self.counter.set(number + 1);
		format!("{prefix}_{number}")
	}

	fn expand(&self, node: Node) -> Node {
		match node {
			// the parser already reads `f = {it*2}` as the definition `f := {it*2}`
			Node::Key(target, Op::Assign | Op::Define, value) if matches!(target.drop_meta(), Node::Symbol(_)) && arrow_lambda(&value).is_some() => {
				let Node::Symbol(name) = target.drop_meta() else { unreachable!("guarded") };
				let lambda = arrow_lambda(&value).expect("guarded");
				definition(name, Lambda { body: self.expand(lambda.body), ..lambda })
			}
			Node::Key(receiver, Op::Dot, method) if self.iteration_method(&method).is_some() => {
				let (iteration, mut arguments) = self.iteration_method(&method).expect("guarded");
				let function = arguments.pop().expect("a function");
				let extras = arguments.into_iter().map(|argument| self.expand(argument)).collect();
				self.iterate(iteration, self.expand(*receiver), extras, self.expand(function))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				self.immediate_call(&items).or_else(|| self.iteration_call(&items, &bracket, &separator)).or_else(|| self.escaping_lambda(&items, &bracket, &separator)).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	/// The iteration a word names, unless the program defines a function of that name
	fn iteration_of(&self, word: &Node) -> Option<&'static Iteration> {
		let Node::Symbol(name) = word.drop_meta() else { return None };
		ITERATIONS.iter().find(|iteration| iteration.word == name && !self.context.user_functions.contains_key(name))
	}

	/// `word(extras, function)` after a dot: the iteration and its arguments, the function last
	fn iteration_method(&self, method: &Node) -> Option<(&'static Iteration, Vec<Node>)> {
		let Node::List(items, _, _) = method.drop_meta() else { return None };
		let (word, arguments) = items.split_first()?;
		let iteration = self.iteration_of(word)?;
		(arguments.len() == iteration.extra_arguments + 1).then(|| (iteration, arguments.to_vec()))
	}

	/// `{x*x}(x=5)`: an anonymous function called with its bindings
	fn immediate_call(&self, items: &[Node]) -> Option<Node> {
		let [block, arguments] = items else { return None };
		let lambda = block_lambda(block).or_else(|| arrow_lambda(block))?;
		let Node::List(entries, Bracket::Round, _) = arguments.drop_meta() else { return None };
		let bindings: Vec<(String, Node)> = entries
			.iter()
			.filter_map(|entry| match entry.drop_meta() {
				Node::Key(name, Op::Assign, value) => match name.drop_meta() {
					Node::Symbol(name) => Some((name.clone(), value.as_ref().clone())),
					_ => None,
				},
				_ => None,
			})
			.collect();
		let (params, values) = if bindings.len() == entries.len() && !bindings.is_empty() {
			(bindings.iter().map(|(name, _)| name.clone()).collect(), bindings.into_iter().map(|(_, value)| value).collect())
		} else {
			(lambda.params.clone(), entries.clone())
		};
		let name = self.fresh("lambda");
		let defined = definition(&name, Lambda { params, body: lambda.body });
		Some(Node::List(vec![defined, call(&name, values)], Bracket::Round, Separator::Semicolon))
	}

	/// `map xs f`, `map(xs, f)`, `fold xs 0 f`, and `xs.map {it*it}` (the method word, then the arguments as the next items)
	fn iteration_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let is_call = call_name(items, bracket, separator).is_some();
		let is_prefix = *bracket == Bracket::None && *separator == Separator::Space;
		if !is_call && !is_prefix {
			return None;
		}
		let (head, rest) = items.split_first()?;
		if let Some(iteration) = self.iteration_of(head) {
			let [list, extras @ .., function] = rest else { return None };
			(extras.len() == iteration.extra_arguments).then(|| self.iterate(iteration, list.clone(), extras.to_vec(), function.clone()))
		} else if let Node::Key(receiver, Op::Dot, word) = head.drop_meta() {
			let iteration = self.iteration_of(word)?;
			let [extras @ .., function] = rest else { return None };
			(is_prefix && extras.len() == iteration.extra_arguments).then(|| self.iterate(iteration, receiver.as_ref().clone(), extras.to_vec(), function.clone()))
		} else {
			None
		}
	}

	/// The function applied to the symbols `arguments`: the body of a literal function with its parameters replaced, or the call of a
	/// defined function; anything else cannot be inlined
	fn applied(&self, iteration: &Iteration, function: &Node, arguments: &[Node]) -> Result<Node, Node> {
		match arrow_lambda(function).or_else(|| block_lambda(function)) {
			Some(lambda) if lambda.params.len() == arguments.len() => {
				Ok(lambda.params.iter().zip(arguments).fold(lambda.body, |body, (param, argument)| substitute(body, param, argument)))
			}
			// a block that does not use `it` takes no argument and is run for every item
			Some(lambda) if lambda.params.is_empty() && arguments.len() == 1 => Ok(lambda.body),
			Some(_) => {
				let count = if iteration.function_arguments == 1 { "one argument" } else { "two arguments" };
				Err(Diagnostic::at(function, format!("{} takes a function of {count}", iteration.word)).into_error())
			}
			None => match function.drop_meta() {
				Node::Symbol(name) if self.context.user_functions.contains_key(name) => Ok(call(name, arguments.to_vec())),
				_ => Err(Diagnostic::at(function, NOT_FIRST_CLASS).into_error()),
			},
		}
	}

	/// The loop of an iteration word with the body of a literal function inlined
	fn iterate(&self, iteration: &Iteration, list: Node, extras: Vec<Node>, function: Node) -> Node {
		let names: Vec<(&str, String)> = ["out", "item", "acc", "list", "index", "value"].into_iter().map(|name| (name, self.fresh(&format!("loop_{name}")))).collect();
		let symbol = |name: &str| Node::Symbol(names.iter().find(|(prefix, _)| *prefix == name).expect("a loop name").1.clone());
		let arguments: Vec<Node> = if iteration.function_arguments == 2 { vec![symbol("acc"), symbol("item")] } else { vec![symbol("item")] };
		let applied = match self.applied(iteration, &function, &arguments) {
			Ok(applied) => applied,
			Err(error) => return error,
		};
		let mut template = iteration.template.to_string();
		for (name, fresh) in &names {
			template = replace_word(&template, name, fresh);
		}
		let mut program = substitute(substitute(parse(&template), LIST_PLACEHOLDER, &list), CALL_PLACEHOLDER, &applied);
		if let Some(start) = extras.first() {
			program = substitute(program, START_PLACEHOLDER, start);
		}
		program
	}

	/// A lambda given as an argument of a call that is not inlined
	fn escaping_lambda(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		call_name(items, bracket, separator)?;
		let escaped = items[1..].iter().find(|argument| arrow_lambda(argument).is_some())?;
		Some(Diagnostic::at(escaped, NOT_FIRST_CLASS).into_error())
	}
}

/// `text` with every whole word `word` replaced (a word is a run of letters, digits and `_`)
fn replace_word(text: &str, word: &str, replacement: &str) -> String {
	let mut result = String::new();
	let mut current = String::new();
	let mut flush = |current: &mut String, result: &mut String| {
		result.push_str(if current == word { replacement } else { current });
		current.clear();
	};
	for character in text.chars() {
		if character.is_alphanumeric() || character == '_' {
			current.push(character);
		} else {
			flush(&mut current, &mut result);
			result.push(character);
		}
	}
	flush(&mut current, &mut result);
	result
}
