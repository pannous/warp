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

/// The right operand of a single `|` as the parser marks it: a bare word may name a function
pub fn pipe_stage(stage: Node) -> Node {
	match stage.drop_meta() {
		Node::Symbol(_) => Node::Meta { node: Box::new(stage), data: Box::new(Node::key(PIPE_STAGE, Node::True)) },
		_ => stage,
	}
}

pub fn is_pipe_stage(node: &Node) -> bool {
	matches!(node, Node::Meta { data, .. } if matches!(data.as_ref(), Node::Key(key, _, _) if key.name() == PIPE_STAGE))
}

pub fn lower(node: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let mut variables = HashSet::new();
	node.visit(&mut |part| {
		if let Node::Key(target, Op::Assign, _) = part {
			if let Node::Symbol(name) = target.drop_meta() {
				variables.insert(name.clone());
			}
		}
	});
	Pipes { context, variables }.rewrite(node)
}

struct Pipes {
	context: Context,
	/// Assigned names: a variable after `|` is an operand of the logical or
	variables: HashSet<String>,
}

impl Pipes {
	fn rewrite(&self, node: Node) -> Node {
		match node {
			Node::Key(value, Op::Or, stage) if self.function(&stage).is_some() => {
				let name = self.function(&stage).expect("guarded");
				called(name, self.rewrite(*value))
			}
			Node::List(mut items, Bracket::None, Separator::Space) if self.pipes_braceless_call(&items) => {
				let Some(Node::Key(argument, Op::Or, stage)) = items.pop().map(|last| last.drop_meta().clone()) else { unreachable!("guarded") };
				items.push(*argument);
				let name = self.function(&stage).expect("guarded");
				called(name, self.rewrite(Node::List(items, Bracket::None, Separator::Space)))
			}
			other => other.map_children(|child| self.rewrite(child)),
		}
	}

	/// The function a marked stage names: the word itself, unless it is a variable
	fn function(&self, stage: &Node) -> Option<Node> {
		let Node::Meta { node, .. } = stage else { return None };
		let Node::Symbol(name) = node.drop_meta() else { return None };
		let is_user_function = self.context.user_functions.get(name).is_some_and(|function| !function.params.is_empty());
		let is_word = !self.variables.contains(name) && is_function_word(name, &self.context);
		(is_pipe_stage(stage) && (is_user_function || is_word)).then(|| node.as_ref().clone())
	}

	/// `square 2 | root`: a braceless call of a function whose last argument pipes into a function
	fn pipes_braceless_call(&self, items: &[Node]) -> bool {
		let head_is_function = match items.first().map(Node::drop_meta) {
			Some(Node::Symbol(name)) => self.context.user_functions.contains_key(name) || is_function_word(name, &self.context),
			_ => false,
		};
		head_is_function && items.len() >= 2 && matches!(items.last().map(Node::drop_meta), Some(Node::Key(_, Op::Or, stage)) if self.function(stage).is_some())
	}
}

fn is_function_word(name: &str, context: &Context) -> bool {
	FUNCTION_WORDS.contains(&name)
		|| crate::library_words::is_library_word(name)
		|| crate::wasm_emitter::text_builtins::is_text_builtin(name)
		|| counting_function(name, context).is_some()
		|| crate::ffi::get_ffi_signature(name).is_some()
}

fn called(function: Node, argument: Node) -> Node {
	Node::List(vec![function, argument], Bracket::Round, Separator::None)
}
