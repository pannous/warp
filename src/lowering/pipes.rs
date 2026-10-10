//! `x | f` is the call `f(x)` when f names a function (D6: "the pipe operator is just an operator that behaves
//! differently with different types", wiki/pipe.md); between values `|` stays the logical or. The parser marks the bare
//! word after a single `|` (pipe_stage); this pass decides. A pipe after a braceless call takes the call's result:
//! `square 2 | root` is `root(square(2))`, `fetch url | trim` trims the fetched text, as in a shell.
//! `then` pipes the same way into a function missing its argument (`xs then sort`, P158); otherwise it is the
//! condition, also without `if`.

use super::lambdas::IMPLICIT_PARAMETER;
use super::nodes::{if_then, key};
use crate::analyzer::counting_function;
use crate::context::Context;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

/// Meta key on the word after a single `|`
const PIPE_STAGE: &str = "pipe stage";
/// Words that are functions besides the library words, text builtins and C functions
const FUNCTION_WORDS: [&str; 7] = ["print", "puts", "fetch", "ceil", "floor", "round", "abs"];
/// Prefix operators spelled as words, a stage alone: `4 | sqrt` is √4 (the parser reads the word as the operator
/// without its operand)
const OPERATOR_STAGES: [Op; 3] = [Op::Sqrt, Op::Cbrt, Op::Abs];

/// May the right operand of a single `|` be a stage: a bare word (it may name a function) or a word operator alone
pub fn may_be_stage(operand: &Node) -> bool {
	matches!(operand.drop_meta(), Node::Symbol(_)) || operator_stage(operand).is_some()
}

/// The right operand of a single `|` as the parser marks it (may_be_stage)
pub fn pipe_stage(stage: Node) -> Node {
	Node::Meta { node: Box::new(stage), data: Box::new(Node::key(PIPE_STAGE, Node::True)) }
}

/// `sqrt`, `abs` alone: the operator
fn operator_stage(node: &Node) -> Option<Op> {
	match node.drop_meta() {
		Node::Key(left, op, right) if OPERATOR_STAGES.contains(op) && matches!(left.drop_meta(), Node::Empty) && matches!(right.drop_meta(), Node::Empty) => Some(*op),
		_ => None,
	}
}

pub fn is_pipe_stage(node: &Node) -> bool {
	matches!(node, Node::Meta { data, .. } if matches!(data.as_ref(), Node::Key(key, _, _) if key.name() == PIPE_STAGE))
}

pub fn lower(node: Node) -> Node {
	let context = crate::analyzer::function_context(&node);
	let mut variables = HashSet::new();
	let mut spaced_functions = HashSet::new();
	node.visit(&mut |part| match part {
		Node::Key(target, Op::Assign, _) => {
			if let Node::Symbol(name) = target.drop_meta() {
				variables.insert(name.clone());
			}
		}
		Node::List(items, Bracket::None, Separator::Space) => spaced_functions.extend(spaced_function(items)),
		_ => {}
	});
	Pipes { context, variables, spaced_functions }.rewrite(node)
}

struct Pipes {
	context: Context,
	/// Assigned names: a variable after `|` is an operand of the logical or
	variables: HashSet<String>,
	/// `square x := x*x`: functions defined in the spaced form, which lower_spaced_definitions rewrites only later
	spaced_functions: HashSet<String>,
}

impl Pipes {
	fn rewrite(&self, node: Node) -> Node {
		match node {
			// `sum|print`: functions on both sides compose, `it => print(sum(it))` (wiki/pipe.md, card pipe-compose)
			Node::Key(value, Op::Or, stage) if self.is_stage(&stage) && self.is_function_chain(&value) => {
				let it = Node::Symbol(IMPLICIT_PARAMETER.to_string());
				key(it.clone(), Op::FatArrow, self.applied(&stage, self.chain_applied(*value, it)))
			}
			// `sum|print 1 2 3` parses as `sum | (print 1 2 3)`: the composition applied, `print(sum([1 2 3]))` (card pipe-applied)
			Node::Key(value, Op::Or, applied) if self.is_function_chain(&value) && self.applied_stage(&applied).is_some() => {
				let (stage, argument) = self.applied_stage(&applied).expect("guarded");
				self.applied(&stage, self.chain_applied(*value, self.rewrite(argument)))
			}
			Node::Key(value, Op::Or, stage) if self.is_stage(&stage) => self.applied(&stage, self.rewrite(*value)),
			// `cond then a else b`: the condition, also without `if` (P158)
			Node::Key(then, Op::Else, otherwise) if is_bare_then(&then) => {
				let Node::Key(condition, _, body) = then.drop_meta().clone() else { unreachable!("guarded") };
				let condition = if_then(self.rewrite(*condition), self.rewrite(*body));
				key(condition, Op::Else, self.rewrite(*otherwise))
			}
			// `3 then f then g` parses as `3 then (f then g)`: a pipeline reads left to right, `(3 then f) then g`
			Node::Key(value, Op::Then, stage) if !is_if_head(&value) && is_bare_then(&stage) => {
				let Node::Key(first, _, rest) = stage.drop_meta().clone() else { unreachable!("guarded") };
				self.rewrite(Node::Key(Box::new(Node::Key(value, Op::Then, first)), Op::Then, rest))
			}
			Node::Key(value, Op::Then, stage) if !is_if_head(&value) => self.then_reading(self.rewrite(*value), self.rewrite(*stage)),
			// `square numbers then filter(p)`: the braceless call is the piped value
			Node::List(mut items, Bracket::None, Separator::Space) if self.then_pipes_braceless_call(&items) => {
				let Some(Node::Key(argument, Op::Then, stage)) = items.pop().map(|last| last.drop_meta().clone()) else { unreachable!("guarded") };
				items.push(*argument);
				let value = self.rewrite(Node::List(items, Bracket::None, Separator::Space));
				self.then_call(&self.rewrite(*stage), &value).expect("guarded")
			}
			Node::List(mut items, Bracket::None, Separator::Space) if self.pipes_braceless_call(&items) => {
				let Some(Node::Key(argument, Op::Or, stage)) = items.pop().map(|last| last.drop_meta().clone()) else { unreachable!("guarded") };
				items.push(*argument);
				self.applied(&stage, self.rewrite(Node::List(items, Bracket::None, Separator::Space)))
			}
			other => other.map_children(|child| self.rewrite(child)),
		}
	}

	/// `value then stage` without `if` (P158): the call when the stage is a function missing its argument and the
	/// value no truth value, else the condition; a truth value before a function stays the condition, with a note
	fn then_reading(&self, value: Node, stage: Node) -> Node {
		match self.then_call(&stage, &value) {
			Some(_) if is_truth_value(&value) => {
				let written = format!("{} then {}", value.serialize(), stage.serialize());
				crate::normalize::set_position_of(&value);
				crate::normalize::advise(&written, &format!("{} |> {}", value.serialize(), stage.serialize()), "then after a comparison is the condition; to pipe the truth value write |>");
				if_then(value, stage)
			}
			Some(call) => call,
			None => if_then(value, stage),
		}
	}

	/// `then sort`, `then filter(p)`: the call of the function with the value as its first argument (as `|>`), when
	/// the stage names a function and leaves out an argument
	fn then_call(&self, stage: &Node, value: &Node) -> Option<Node> {
		if let Some(op) = operator_stage(stage) {
			return Some(key(Node::Empty, op, grouped(value.clone())));
		}
		let pipes = match stage.drop_meta() {
			Node::Symbol(name) => self.names_function(name, 0),
			Node::List(items, Bracket::Round, Separator::None) => match items.first().map(Node::drop_meta) {
				Some(Node::Symbol(name)) => self.leaves_out_an_argument(name, items.len() - 1),
				_ => false,
			},
			_ => false,
		};
		pipes.then(|| crate::warp_parser::piped(value.clone(), stage.clone()))
	}

	/// Does `name` call a function that takes more than `given` arguments (a word's own count is not known: any)
	fn names_function(&self, name: &str, given: usize) -> bool {
		if let Some(function) = self.context.user_functions.get(name) {
			return function.params.len() > given;
		}
		self.spaced_functions.contains(name) || (!self.variables.contains(name) && is_function_word(name, &self.context))
	}

	/// `filter(p)`: a call of anything but a variable, short of an argument when it is the program's function
	fn leaves_out_an_argument(&self, name: &str, given: usize) -> bool {
		match self.context.user_functions.get(name) {
			Some(function) => function.params.len() > given,
			None => !self.variables.contains(name),
		}
	}

	/// `square numbers then filter(p)`: a braceless call whose last argument pipes on with `then`
	fn then_pipes_braceless_call(&self, items: &[Node]) -> bool {
		let head_is_function = matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(name)) if self.names_function(name, 0));
		head_is_function && items.len() >= 2 && matches!(items.last().map(Node::drop_meta), Some(Node::Key(argument, Op::Then, stage)) if !is_truth_value(argument) && self.then_call(stage, argument).is_some())
	}

	/// A marked stage that pipes: a function or a word operator
	fn is_stage(&self, stage: &Node) -> bool {
		self.function(stage).is_some() || (is_pipe_stage(stage) && operator_stage(stage).is_some())
	}

	/// `sum`, `square|sqrt`: a function, or functions piped into each other
	fn is_function_chain(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Symbol(name) => self.names_function(name, 0),
			Node::Key(value, Op::Or, stage) => self.is_stage(stage) && self.is_function_chain(value),
			_ => false,
		}
	}

	/// `print 1 2 3` (parsed as the call `print((1 2 3))`), `sq 1 2` (parsed as `((sq 1) 2)`), `√3` after a function
	/// chain's `|`: the stage (marked as the parser marks a bare one) and the argument of the whole chain, several
	/// arguments as one list (`sum 1 2 3` sums them)
	fn applied_stage(&self, applied: &Node) -> Option<(Node, Node)> {
		let (stage, mut arguments) = match applied.drop_meta() {
			Node::List(items, Bracket::None, Separator::Space) | Node::List(items, Bracket::Round, Separator::None) if items.len() >= 2 => {
				let mut words: Vec<Node> = items.iter().flat_map(spread).collect();
				(pipe_stage(words.remove(0)), words)
			}
			Node::Key(left, op, operand) if OPERATOR_STAGES.contains(op) && matches!(left.drop_meta(), Node::Empty) && !matches!(operand.drop_meta(), Node::Empty) => {
				(pipe_stage(key(Node::Empty, *op, Node::Empty)), vec![operand.as_ref().clone()])
			}
			_ => return None,
		};
		let argument = if arguments.len() == 1 { arguments.remove(0) } else { Node::List(arguments, Bracket::Square, Separator::Space) };
		self.is_stage(&stage).then_some((stage, argument))
	}

	/// The function chain called with `argument`: `square|sqrt` → `√(square(argument))`
	fn chain_applied(&self, chain: Node, argument: Node) -> Node {
		match chain.drop_meta().clone() {
			Node::Key(value, Op::Or, stage) => self.applied(&stage, self.chain_applied(*value, argument)),
			function => called(function, argument),
		}
	}

	/// The stage applied to the piped value: the call `f(value)`, or the operator `√value`
	fn applied(&self, stage: &Node, value: Node) -> Node {
		match self.function(stage) {
			Some(function) => called(function, value),
			None => key(Node::Empty, operator_stage(stage).expect("a stage"), grouped(value)),
		}
	}

	/// The function a marked stage names: the word itself, unless it is a variable
	fn function(&self, stage: &Node) -> Option<Node> {
		let Node::Meta { node, .. } = stage else { return None };
		let Node::Symbol(name) = node.drop_meta() else { return None };
		let is_user_function = self.context.user_functions.get(name).is_some_and(|function| !function.params.is_empty()) || self.spaced_functions.contains(name);
		let is_word = !self.variables.contains(name) && is_function_word(name, &self.context);
		(is_pipe_stage(stage) && (is_user_function || is_word)).then(|| node.as_ref().clone())
	}

	/// `square 2 | root`: a braceless call of a function whose last argument pipes into a function
	fn pipes_braceless_call(&self, items: &[Node]) -> bool {
		let head_is_function = match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => self.context.user_functions.contains_key(name) || self.spaced_functions.contains(name) || is_function_word(name, &self.context),
			_ => false,
		};
		head_is_function && items.len() >= 2 && matches!(items.last().map(Node::drop_meta), Some(Node::Key(_, Op::Or, stage)) if self.is_stage(stage))
	}
}

/// The name `square x := x*x` defines (`:=` with at least one parameter; `=` stays a possible assignment)
fn spaced_function(items: &[Node]) -> Option<String> {
	let (name, _, rest) = crate::declarations::spaced_definition_words(items)?;
	matches!(rest.first().map(Node::drop_meta), Some(Node::Key(_, Op::Define, _))).then(|| name.name())
}

fn is_function_word(name: &str, context: &Context) -> bool {
	FUNCTION_WORDS.contains(&name)
		|| crate::library_words::is_library_word(name)
		|| crate::wasm_emitter::text_builtins::is_text_builtin(name)
		|| counting_function(name, context).is_some()
		|| crate::ffi::get_ffi_signature(name).is_some()
}

/// `a then b` without `if`
fn is_bare_then(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(head, Op::Then, _) if !is_if_head(head))
}

/// `if c` before `then`
fn is_if_head(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::If, _))
}

/// A comparison or logic: `x > 2`, `a and b`, `not x`, true
fn is_truth_value(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, op, _) => op.is_comparison() || op.is_equality() || op.is_logical() || *op == Op::Not,
		Node::True | Node::False => true,
		_ => false,
	}
}

/// `square 2` as the operand of a prefix operator: `√(square 2)`, not `√ square 2` (which reads as square(√2))
fn grouped(value: Node) -> Node {
	match value {
		Node::List(items, Bracket::None, separator) => Node::List(items, Bracket::Round, separator),
		other => other,
	}
}

/// The words of an unbracketed list, nested ones too (`(sq 1) 2`), any other node alone
fn spread(node: &Node) -> Vec<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, Separator::Space) => items.iter().flat_map(spread).collect(),
		_ => vec![node.clone()],
	}
}

fn called(function: Node, argument: Node) -> Node {
	Node::List(vec![function, argument], Bracket::Round, Separator::None)
}
