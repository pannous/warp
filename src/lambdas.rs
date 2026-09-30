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
const MAP: &str = "map";
const LIST_PLACEHOLDER: &str = "map_list";
const CALL_PLACEHOLDER: &str = "map_call";

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
		Node::Key(left, Op::Arrow | Op::FatArrow, body) => Some(Lambda { params: parameter_names(left)?, body: body.as_ref().clone() }),
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
			Node::Key(receiver, Op::Dot, method) if self.map_method_argument(&method).is_some() => {
				let function = self.map_method_argument(&method).expect("guarded");
				self.map(self.expand(*receiver), self.expand(function))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				self.immediate_call(&items).or_else(|| self.map_call(&items, &bracket, &separator)).or_else(|| self.escaping_lambda(&items, &bracket, &separator)).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	fn map_method_argument(&self, method: &Node) -> Option<Node> {
		let Node::List(items, _, _) = method.drop_meta() else { return None };
		match items.as_slice() {
			[word, function] if matches!(word.drop_meta(), Node::Symbol(name) if name == MAP) && !self.context.user_functions.contains_key(MAP) => Some(function.clone()),
			_ => None,
		}
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

	/// `map xs f`, `map(xs, f)`
	fn map_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let is_map = matches!(items.first()?.drop_meta(), Node::Symbol(name) if name == MAP) && !self.context.user_functions.contains_key(MAP);
		let is_call = call_name(items, bracket, separator).is_some();
		let is_prefix = *bracket == Bracket::None && *separator == Separator::Space;
		match items {
			[_, list, function] if is_map && (is_call || is_prefix) => Some(self.map(list.clone(), function.clone())),
			// `xs.map {it*it}`: the method word, then the block as the next item
			[method, function] if is_prefix && !self.context.user_functions.contains_key(MAP) => match method.drop_meta() {
				Node::Key(receiver, Op::Dot, word) if matches!(word.drop_meta(), Node::Symbol(name) if name == MAP) => Some(self.map(receiver.as_ref().clone(), function.clone())),
				_ => None,
			},
			_ => None,
		}
	}

	/// The loop `(out=[]; for item in list { out.add(f(item)) }; out)` with the body of a literal function inlined
	fn map(&self, list: Node, function: Node) -> Node {
		let (out, item) = (self.fresh("map_out"), self.fresh("map_item"));
		let applied = match arrow_lambda(&function).or_else(|| block_lambda(&function)) {
			Some(lambda) if lambda.params.len() <= 1 => match lambda.params.first() {
				Some(param) => substitute(lambda.body, param, &Node::Symbol(item.clone())),
				None => lambda.body,
			},
			Some(_) => return Diagnostic::at(&function, "map takes a function of one argument").into_error(),
			None => match function.drop_meta() {
				Node::Symbol(name) if self.context.user_functions.contains_key(name) => call(name, vec![Node::Symbol(item.clone())]),
				_ => return Diagnostic::at(&function, NOT_FIRST_CLASS).into_error(),
			},
		};
		let program = parse(&format!("({out}=[]; for {item} in {LIST_PLACEHOLDER} {{ {out}.add({CALL_PLACEHOLDER}) }}; {out})"));
		substitute(substitute(program, LIST_PLACEHOLDER, &list), CALL_PLACEHOLDER, &applied)
	}

	/// A lambda given as an argument of a call that is not inlined
	fn escaping_lambda(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		call_name(items, bracket, separator)?;
		let escaped = items[1..].iter().find(|argument| arrow_lambda(argument).is_some())?;
		Some(Diagnostic::at(escaped, NOT_FIRST_CLASS).into_error())
	}
}
