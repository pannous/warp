//! The basic text and list words (`upper`, `first`, `sum`, `split` …) in their three spellings: `x.word`, `x.word(args)`,
//! `word(x, args)` and `word x args` all become the call `word(x, args)`.
//! Words that need no runtime function are expanded to source here (`first`, `last`, `sum`); the others are calls that the
//! emitter resolves (`RUNTIME_WORDS`). A user function or variable of the same name wins.
//!
//! An unknown `.word` after a name, a text or a list is a loud error (`undefined function: word`), never silent data.

use crate::analyzer::{call_name, counting_method, extract_user_functions, is_append_method};
use crate::context::Context;
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::parse;
use std::cell::Cell;
use std::collections::HashSet;

/// Canonical word and the spellings that mean it
const SYNONYMS: [(&str, &[&str]); 8] = [
	("upper", &["uppercase"]),
	("lower", &["lowercase"]),
	("reverse", &[]),
	("sort", &[]),
	("split", &[]),
	("join", &[]),
	("first", &[]),
	("last", &[]),
];
const SUM: &str = "sum";

/// Words the emitter implements as runtime functions, with the number of arguments including the receiver
pub const RUNTIME_WORDS: [(&str, usize); 6] = [("upper", 1), ("lower", 1), ("reverse", 1), ("sort", 1), ("split", 2), ("join", 2)];

/// Source of the words expanded here; `word_argument` is the receiver, `word_tmp` a temporary that holds it once
const EXPANDED_WORDS: [(&str, &str); 3] = [
	("first", "word_tmp#1"),
	("last", "word_tmp#(count(word_tmp))"),
	(SUM, "(word_sum=0; for word_item in word_tmp {word_sum = word_sum + word_item}; word_sum)"),
];
const RECEIVER_PLACEHOLDER: &str = "word_argument";
const TEMPORARY: &str = "word_tmp";

pub fn is_runtime_word(name: &str) -> bool {
	RUNTIME_WORDS.iter().any(|(word, _)| *word == name)
}

fn canonical_word(name: &str) -> Option<&'static str> {
	SYNONYMS
		.iter()
		.find(|(word, synonyms)| *word == name || synonyms.contains(&name))
		.map(|(word, _)| *word)
		.or((name == SUM).then_some(SUM))
}

fn arity(word: &str) -> usize {
	RUNTIME_WORDS.iter().find(|(name, _)| *name == word).map_or(1, |(_, arity)| *arity)
}

pub fn lower(node: Node) -> Node {
	let mut context = Context::new();
	extract_user_functions(&mut context, &node);
	let mut shadowed: HashSet<String> = context.user_functions.keys().cloned().collect();
	collect_assigned_names(&node, &mut shadowed);
	Lowering { context, shadowed, temporaries: Cell::new(0) }.expand(node)
}

fn collect_assigned_names(node: &Node, names: &mut HashSet<String>) {
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Node::Symbol(name) = target.drop_meta() {
				names.insert(name.clone());
			}
			collect_assigned_names(value, names);
		}
		Node::Key(left, _, right) => {
			collect_assigned_names(left, names);
			collect_assigned_names(right, names);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_assigned_names(item, names)),
		_ => {}
	}
}

struct Lowering {
	context: Context,
	shadowed: HashSet<String>,
	temporaries: Cell<usize>,
}

impl Lowering {
	fn expand(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				self.word_call(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Key(left, Op::Dot, right) => {
				let (left, right) = (self.expand(*left), self.expand_method(*right));
				self.method_call(&left, &right).unwrap_or(Node::Key(Box::new(left), Op::Dot, Box::new(right)))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.expand(*left)), op, Box::new(self.expand(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	/// The word after a dot, `word` or `word(arguments)`: only the arguments are expanded, the word is not a call of its own
	fn expand_method(&self, method: Node) -> Node {
		match method {
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand_method(*node)), data },
			Node::List(items, bracket, separator) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
				let mut items = items.into_iter();
				let word = items.next().expect("guarded");
				Node::List([vec![word], items.map(|argument| self.expand(argument)).collect()].concat(), bracket, separator)
			}
			other => self.expand(other),
		}
	}

	fn library_word(&self, name: &str) -> Option<&'static str> {
		canonical_word(name).filter(|word| !self.shadowed.contains(name) && !self.shadowed.contains(*word))
	}

	/// `word(x, args)` and `word x args`
	fn word_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let head = match items.first()?.drop_meta() {
			Node::Symbol(name) => name,
			_ => return None,
		};
		let word = self.library_word(head)?;
		let is_call = call_name(items, bracket, separator).is_some();
		let is_prefix = *bracket == Bracket::None && *separator == Separator::Space && items.len() > 1;
		if !is_call && !is_prefix {
			return None;
		}
		Some(self.call(word, &items[0], items[1..].to_vec(), is_prefix && !is_call))
	}

	/// `x.word` and `x.word(args)`; an unknown word on a value is an error
	fn method_call(&self, receiver: &Node, method: &Node) -> Option<Node> {
		let (word_node, arguments) = match method.drop_meta() {
			Node::Symbol(_) => (method, vec![]),
			Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => (&items[0], items[1..].to_vec()),
			_ => return None,
		};
		let Node::Symbol(name) = word_node.drop_meta() else { return None };
		if let Some(word) = self.library_word(name) {
			return Some(self.call(word, word_node, [vec![receiver.clone()], arguments].concat(), false));
		}
		let is_known = counting_method(name, &self.context).is_some() || is_append_method(name) || self.context.user_functions.contains_key(name);
		let is_value = matches!(receiver.drop_meta(), Node::Symbol(_) | Node::Text(_) | Node::Char(_) | Node::Number(_) | Node::List(_, Bracket::Square, _));
		(!is_known && is_value).then(|| Diagnostic::at(word_node, format!("undefined function: {name}")).into_error())
	}

	/// The call of `word` with the receiver and its arguments; arguments are the items after the word for the prefix form
	fn call(&self, word: &'static str, head: &Node, arguments: Vec<Node>, is_prefix: bool) -> Node {
		let arguments = if is_prefix { merge_prefix_arguments(word, arguments) } else { arguments };
		let wanted = arity(word);
		if arguments.len() != wanted {
			let plural = if wanted == 1 { "" } else { "s" };
			return Diagnostic::at(head, format!("{word} takes {wanted} argument{plural}, got {}", arguments.len())).into_error();
		}
		match EXPANDED_WORDS.iter().find(|(name, _)| *name == word) {
			Some((_, template)) => self.expanded(template, arguments.into_iter().next().unwrap_or(Node::Empty)),
			None => {
				let name = if matches!(head.drop_meta(), Node::Symbol(written) if written == word) { head.clone() } else { Node::Symbol(word.to_string()) };
				Node::List([vec![name], arguments].concat(), Bracket::Round, Separator::None)
			}
		}
	}

	fn expanded(&self, template: &str, receiver: Node) -> Node {
		let number = self.temporaries.get();
		self.temporaries.set(number + 1);
		let temporary = format!("{TEMPORARY}_{number}");
		let body = template.replace(TEMPORARY, &temporary);
		let program = parse(&format!("({temporary}={RECEIVER_PLACEHOLDER}; {body})"));
		substitute(program, RECEIVER_PLACEHOLDER, &receiver)
	}
}

/// `first [1 2 3]` has one argument, `join [1 2] ","` two: a prefix call with more items than the word takes keeps the rest together
fn merge_prefix_arguments(word: &str, arguments: Vec<Node>) -> Vec<Node> {
	let wanted = arity(word);
	if arguments.len() <= wanted {
		return arguments;
	}
	let (kept, rest) = arguments.split_at(wanted - 1);
	[kept.to_vec(), vec![Node::List(rest.to_vec(), Bracket::None, Separator::Space)]].concat()
}

fn substitute(node: Node, placeholder: &str, replacement: &Node) -> Node {
	match node {
		Node::Symbol(name) if name == placeholder => replacement.clone(),
		Node::Key(left, op, right) => Node::Key(Box::new(substitute(*left, placeholder, replacement)), op, Box::new(substitute(*right, placeholder, replacement))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| substitute(item, placeholder, replacement)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(substitute(*node, placeholder, replacement)), data },
		other => other,
	}
}
