//! `x | f` is the call `f(x)` when f names a function (D6: "the pipe operator is just an operator that behaves
//! differently with different types", wiki/pipe.md); between values `|` stays the logical or. The parser marks the bare
//! word after a single `|` (pipe_stage); this pass decides. A pipe after a braceless call takes the call's result:
//! `square 2 | root` is `root(square(2))`, `fetch url | trim` trims the fetched text, as in a shell.

use crate::analyzer::{counting_function, extract_user_functions};
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
		Node::Key(left, op, right) if OPERATOR_STAGES.contains(op) && matches!(left.drop_meta(), Node::Empty) && matches!(right.drop_meta(), Node::Empty) => Some(op.clone()),
		_ => None,
	}
}

pub fn is_pipe_stage(node: &Node) -> bool {
	matches!(node, Node::Meta { data, .. } if matches!(data.as_ref(), Node::Key(key, _, _) if key.name() == PIPE_STAGE))
}

pub fn lower(node: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
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
			Node::Key(value, Op::Or, stage) if self.is_stage(&stage) => self.applied(&stage, self.rewrite(*value)),
			Node::List(mut items, Bracket::None, Separator::Space) if self.pipes_braceless_call(&items) => {
				let Some(Node::Key(argument, Op::Or, stage)) = items.pop().map(|last| last.drop_meta().clone()) else { unreachable!("guarded") };
				items.push(*argument);
				self.applied(&stage, self.rewrite(Node::List(items, Bracket::None, Separator::Space)))
			}
			other => other.map_children(|child| self.rewrite(child)),
		}
	}

	/// A marked stage that pipes: a function or a word operator
	fn is_stage(&self, stage: &Node) -> bool {
		self.function(stage).is_some() || (is_pipe_stage(stage) && operator_stage(stage).is_some())
	}

	/// The stage applied to the piped value: the call `f(value)`, or the operator `√value`
	fn applied(&self, stage: &Node, value: Node) -> Node {
		match self.function(stage) {
			Some(function) => called(function, value),
			None => Node::Key(Box::new(Node::Empty), operator_stage(stage).expect("a stage"), Box::new(grouped(value))),
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

/// `square 2` as the operand of a prefix operator: `√(square 2)`, not `√ square 2` (which reads as square(√2))
fn grouped(value: Node) -> Node {
	match value {
		Node::List(items, Bracket::None, separator) => Node::List(items, Bracket::Round, separator),
		other => other,
	}
}

fn called(function: Node, argument: Node) -> Node {
	Node::List(vec![function, argument], Bracket::Round, Separator::None)
}
