//! Builtins `min(a, b, …)` and `max(a, b, …)`: lowered to comparisons, so exact and float operands
//! share the arithmetic emitters. Arguments beyond plain values and arithmetic are computed once into temporaries.
//! One list argument is the list of candidates: `max([1 5 2])`; a list variable `max(xs)` is folded at runtime.

use crate::analyzer::{call_name, extract_user_functions};
use crate::context::Context;
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::warp_parser::parse;
use crate::operators::Op;

const EXTREMA: [(&str, Op); 2] = [("min", Op::Lt), ("max", Op::Gt)];
const MIN_ARGUMENTS: usize = 2;
const TEMPORARY_PREFIX: &str = "extremum_argument_";

/// Pseudo-call the emitter turns into the runtime error `<extremum> of an empty list`
pub const EMPTY_EXTREMUM_CALL: &str = "empty_extremum";
pub const EMPTY_LIST_ERRORS: [(&str, &str); 3] =
	[("min", "min_of_an_empty_list"), ("max", "max_of_an_empty_list"), ("reduce", "reduce_of_an_empty_list")];

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&EXTREMA.map(|(name, _)| name)) {
		return node;
	}
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let builtins: Vec<(&str, Op)> = EXTREMA.into_iter().filter(|(name, _)| !context.user_functions.contains_key(*name)).collect();
	if builtins.is_empty() {
		return node;
	}
	Lowering { builtins, temporaries: 0 }.expand(node)
}

struct Lowering<'a> {
	builtins: Vec<(&'a str, Op)>,
	temporaries: usize,
}

impl Lowering<'_> {
	fn expand(&mut self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				self.extremum(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			// `xs.max()`: the call `max(xs)`, not the max of no values
			Node::Key(receiver, Op::Dot, method) if self.extremum_method(&method).is_some() => {
				let (name, arguments) = self.extremum_method(&method).expect("guarded");
				self.expand(Node::List([vec![name, *receiver], arguments].concat(), Bracket::Round, Separator::None))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	/// `max()`, `max(k)` after a dot: the extremum word and its arguments
	fn extremum_method(&self, method: &Node) -> Option<(Node, Vec<Node>)> {
		let Node::List(items, Bracket::Round, _) = method.drop_meta() else { return None };
		let (word, arguments) = items.split_first()?;
		let is_extremum = matches!(word.drop_meta(), Node::Symbol(name) if self.builtins.iter().any(|(builtin, _)| builtin == name));
		is_extremum.then(|| (word.clone(), arguments.to_vec()))
	}

	fn extremum(&mut self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		// `max xs`, `max [1 5 2]`: one argument without parentheses is a call too
		let prefix = matches!((bracket, separator, items), (Bracket::None, Separator::Space, [head, _]) if matches!(head.drop_meta(), Node::Symbol(_)));
		let head = items.first()?.drop_meta().name();
		let name = call_name(items, bracket, separator).map(str::to_string).or_else(|| prefix.then_some(head))?;
		let name = name.as_str();
		let (_, better) = *self.builtins.iter().find(|(builtin, _)| *builtin == name)?;
		let listed = list_literal_items(&items[1..]);
		let arguments = listed.unwrap_or(&items[1..]);
		if let ([list], None) = (arguments, listed) {
			return Some(match list.drop_meta() {
				Node::Symbol(list) => fold_list_at_runtime(name, list, &better),
				value if is_plain(value) => Diagnostic::at(&items[0], format!("{name} takes at least {MIN_ARGUMENTS} arguments or one list, got 1")).into_error(),
				_ => {
					let temporary = self.temporary();
					let binding = Node::Key(Box::new(Node::Symbol(temporary.clone())), Op::Assign, Box::new(list.clone()));
					with_bindings(vec![binding], fold_list_at_runtime(name, &temporary, &better))
				}
			});
		}
		if arguments.is_empty() {
			return Some(Diagnostic::at(&items[0], format!("{name} of an empty list")).into_error());
		}
		let (bindings, arguments) = self.bind_once(arguments);
		let chosen = arguments[1..].iter().fold(arguments[0].clone(), |best, next| {
			let comparison = Node::Key(Box::new(best.clone()), better, Box::new(next.clone()));
			let branches = Node::Key(Box::new(best), Op::Colon, Box::new(next.clone()));
			Node::Key(Box::new(comparison), Op::Question, Box::new(branches))
		});
		Some(with_bindings(bindings, chosen))
	}

	/// The comparisons repeat their operands: an argument that is more than a plain value or arithmetic
	/// (a call, an index, a nested min) is computed once into a temporary local
	fn bind_once(&mut self, arguments: &[Node]) -> (Vec<Node>, Vec<Node>) {
		let mut bindings = vec![];
		let operands = arguments.iter().map(|argument| {
			if is_plain(argument) {
				return argument.clone();
			}
			let temporary = Node::Symbol(self.temporary());
			bindings.push(Node::Key(Box::new(temporary.clone()), Op::Assign, Box::new(argument.clone())));
			temporary
		}).collect();
		(bindings, operands)
	}

	fn temporary(&mut self) -> String {
		self.temporaries += 1;
		format!("{TEMPORARY_PREFIX}{}", self.temporaries)
	}
}

/// `(t1 = a; t2 = b; value)`, or just the value without bindings
pub(crate) fn with_bindings(mut bindings: Vec<Node>, value: Node) -> Node {
	if bindings.is_empty() {
		return value;
	}
	bindings.push(value);
	Node::List(bindings, Bracket::Round, Separator::Semicolon)
}

/// The items of the one list literal given instead of several arguments; ø is the empty list
fn list_literal_items(arguments: &[Node]) -> Option<&[Node]> {
	match arguments {
		[single] => match single.drop_meta() {
			Node::List(items, Bracket::Square, _) => Some(items),
			Node::Empty => Some(&[]),
			_ => None,
		},
		_ => None,
	}
}

/// `max(xs)` for a list variable: the first item, improved by every later one; an empty list is an error
fn fold_list_at_runtime(name: &str, list: &str, better: &Op) -> Node {
	parse(&format!(
		"(if count({list}) == 0 then {EMPTY_EXTREMUM_CALL}({name}) else (extremum_best={list}#1; for extremum_item in {list} {{ extremum_best = if extremum_item {better} extremum_best then extremum_item else extremum_best }}; extremum_best))"
	))
}

/// A value or arithmetic on values: cheap and side-effect free to repeat in the comparisons
pub(crate) fn is_plain(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Number(_) | Node::Symbol(_) | Node::True | Node::False | Node::Empty => true,
		Node::Key(left, op, right) => {
			(op.is_arithmetic() || op.is_comparison() || matches!(op, Op::Question | Op::Colon)) && is_plain(left) && is_plain(right)
		}
		_ => false,
	}
}
