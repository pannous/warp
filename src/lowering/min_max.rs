//! Builtins `min(a, b, …)` and `max(a, b, …)`: lowered to comparisons, so exact and float operands
//! share the arithmetic emitters. Arguments beyond plain values and arithmetic are computed once into temporaries.
//! One list argument is the list of candidates: `max([1 5 2])`; a list variable `max(xs)` is folded at runtime.

use super::nodes::{Counter, key};
use crate::analyzer::call_name;
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::warp_parser::parse;
use crate::operators::Op;

const EXTREMA: [(&str, Op); 2] = [("min", Op::Lt), ("max", Op::Gt)];
const MIN_ARGUMENTS: usize = 2;
const TEMPORARY_BASE: &str = "extremum";
/// `extremum_best` of the runtime fold becomes the temporary `extremum·best·3`
const FOLD_PREFIX: &str = "extremum_";
const FOLD_LIST: &str = "fold_list_placeholder";

/// Pseudo-call the emitter turns into the runtime error `<extremum> of an empty list`
pub const EMPTY_EXTREMUM_CALL: &str = "empty_extremum";
pub const EMPTY_LIST_ERRORS: [(&str, &str); 3] =
	[("min", "min_of_an_empty_list"), ("max", "max_of_an_empty_list"), ("reduce", "reduce_of_an_empty_list")];

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&EXTREMA.map(|(name, _)| name)) {
		return node;
	}
	let context = crate::analyzer::function_context(&node);
	let builtins: Vec<(&str, Op)> = EXTREMA.into_iter().filter(|(name, _)| !context.user_functions.contains_key(*name)).collect();
	if builtins.is_empty() {
		return node;
	}
	Lowering { builtins, temporaries: Counter::starting_at(1) }.expand(node)
}

struct Lowering<'a> {
	builtins: Vec<(&'a str, Op)>,
	temporaries: Counter,
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
			Node::Key(left, op, right) => key(self.expand(*left), op, self.expand(*right)),
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
				Node::Symbol(_) => self.fold_list_at_runtime(name, list, &better),
				value if is_plain(value) => Diagnostic::at(&items[0], format!("{name} takes at least {MIN_ARGUMENTS} arguments or one list, got 1")).into_error(),
				_ => {
					let temporary = Node::Symbol(self.temporary());
					let binding = key(temporary.clone(), Op::Assign, list.clone());
					with_bindings(vec![binding], self.fold_list_at_runtime(name, &temporary, &better))
				}
			});
		}
		if arguments.is_empty() {
			return Some(Diagnostic::at(&items[0], format!("{name} of an empty list")).into_error());
		}
		let (mut bindings, arguments) = self.bind_once(arguments);
		let mut best = arguments[0].clone();
		for next in &arguments[1..] {
			// each comparison names the best so far twice: unbound, n arguments would make 2ⁿ nodes
			if matches!(best.drop_meta(), Node::Key(_, Op::Question, _)) {
				best = self.bind(&mut bindings, best);
			}
			let comparison = key(best.clone(), better, next.clone());
			let branches = key(best, Op::Colon, next.clone());
			best = key(comparison, Op::Question, branches);
		}
		Some(with_bindings(bindings, best))
	}

	/// `temporary = value` appended to the bindings; the temporary stands for the value
	fn bind(&mut self, bindings: &mut Vec<Node>, value: Node) -> Node {
		let temporary = Node::Symbol(self.temporary());
		bindings.push(key(temporary.clone(), Op::Assign, value));
		temporary
	}

	/// The comparisons repeat their operands: an argument that is more than a plain value or arithmetic
	/// (a call, an index, a nested min) is computed once into a temporary local
	fn bind_once(&mut self, arguments: &[Node]) -> (Vec<Node>, Vec<Node>) {
		let mut bindings = vec![];
		let operands = arguments.iter().map(|argument| {
			if is_plain(argument) {
				return argument.clone();
			}
			self.bind(&mut bindings, argument.clone())
		}).collect();
		(bindings, operands)
	}

	/// `max(xs)` for a list variable: the first item, improved by every later one; an empty list is an error
	fn fold_list_at_runtime(&mut self, name: &str, list: &Node, better: &Op) -> Node {
		let template = parse(&format!(
			"(if count({FOLD_LIST}) == 0 then {EMPTY_EXTREMUM_CALL}({name}) else ({FOLD_PREFIX}best={FOLD_LIST}#1; for {FOLD_PREFIX}item in {FOLD_LIST} {{ {FOLD_PREFIX}best = if {FOLD_PREFIX}item {better} {FOLD_PREFIX}best then {FOLD_PREFIX}item else {FOLD_PREFIX}best }}; {FOLD_PREFIX}best))"
		));
		crate::library_words::substitute(crate::library_words::named_apart(template, FOLD_PREFIX, TEMPORARY_BASE, self.temporaries.next_number()), FOLD_LIST, list)
	}

	fn temporary(&mut self) -> String {
		crate::library_words::temporary_name(&[TEMPORARY_BASE, "argument", &self.temporaries.next_number().to_string()])
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
