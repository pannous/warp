//! Keywords (P165, user): hard keywords — control flow, declarations, literal values, modules — are never redefined.
//! Every other keyword-like word is soft: it may name a local variable, parameter, field or method, where the program's
//! name wins with a got-it note ("send is a keyword; here it is your variable"); redefining one at the top level is a
//! loud error. Passes that give a soft keyword its meaning ask `program_names` whether the program took the word.

use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node};
use crate::operators::Op;

pub const HARD_KEYWORDS: [&str; 26] = [
	"if", "then", "else", "while", "for", "in", "return", "break", "continue", // control flow
	"def", "fun", "fn", "class", "var", "let", "const", "val", "global", // declarations
	"true", "false", "ø", "null", "nil", // literal values
	"use", "import", "include", // modules
];
/// emit's aliases fire, trigger and signal stay out: a program may define them (P163, test_event_footguns; asked)
pub const SOFT_KEYWORDS: [&str; 12] = ["emit", "send", "broadcast", "every", "whenever", "init", "new", "root", "listeners", "stored", "undo", "redo"];
/// Keyword-like words in neither list: the playground editor colors them like soft keywords (web/playground/build.sh
/// makes keywords.js). Display only: P165 gives them no meaning; moving one into a list above is a user decision
pub const HIGHLIGHTED_WORDS: [&str; 59] = ["try", "catch", "except", "switch", "match", "case", "do", "yield", "await", "go", "elif", "elsif", "elseif", "end", "until", "law", "on", "once", "within", "after", "of", "from", "to", "upto", "times", "is", "as", "with", "extends", "mixin", "static", "trait", "interface", "protocol", "struct", "record", "operator", "nonlocal", "lambda", "define", "function", "func", "result", "print", "assert", "linear", "shared", "ref", "meta", "raise", "throw", "require", "and", "or", "not", "xor", "fire", "trigger", "signal"];
const TOPIC: &str = "soft-keyword";

pub fn lower(program: Node) -> Node {
	if let Some(error) = first_refused(&program, true) {
		return error;
	}
	note_local_names(&program);
	program
}

/// Whether the program names `word` itself: a variable, parameter, field or function of that name
pub fn program_names(program: &Node, word: &str) -> bool {
	let mut named = false;
	walk_names(program, &mut |name, _| named |= name == word);
	named
}

/// A hard keyword assigned anywhere, a soft one defined at the top level: the loud error
fn first_refused(node: &Node, top_level: bool) -> Option<Node> {
	if let Some(name) = defined_name(node) {
		let refused = HARD_KEYWORDS.contains(&name.as_str()) || top_level && SOFT_KEYWORDS.contains(&name.as_str());
		if refused {
			let fix = if top_level && SOFT_KEYWORDS.contains(&name.as_str()) { "give it another name; a local variable, parameter or field may use it" } else { "give it another name" };
			return Some(Diagnostic::at(node, format!("{name} is a keyword: {fix}")).into_error());
		}
	}
	match node.drop_meta() {
		// the statements of the program, or of a block at its top level, stay at the top level
		Node::List(items, Bracket::None | Bracket::Curly, _) if top_level => items.iter().find_map(|item| first_refused(item, true)),
		Node::Key(_, Op::Define | Op::Assign, body) if defined_name(node).is_some() => first_refused(body, false),
		Node::Key(left, _, right) => first_refused(left, false).or_else(|| first_refused(right, false)),
		Node::List(items, _, _) => items.iter().find_map(|item| first_refused(item, false)),
		_ => None,
	}
}

/// `x = …`, `f(x) := …`, `def f(x) {…}`: the name defined
fn defined_name(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Key(target, Op::Define | Op::Assign, _) => match target.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			Node::List(items, Bracket::Round, _) => items.first().and_then(symbol_name),
			_ => None,
		},
		// `def ((init ø) {1})`: the name leads the head
		Node::List(words, _, _) if words.len() > 1 && words.first().is_some_and(is_function_keyword) => {
			Some(crate::lowering::class_methods::leading_name(&words[1])).filter(|name| !name.is_empty())
		}
		_ => None,
	}
}

fn symbol_name(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		_ => None,
	}
}

fn is_function_keyword(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word))
}

/// The got-it note for each soft keyword the program uses as a local name
fn note_local_names(program: &Node) {
	walk_names(program, &mut |name, role| {
		if SOFT_KEYWORDS.contains(&name) {
			crate::diagnostic::educate_once(&format!("{TOPIC}-{name}"), name, name, &format!("{name} is a keyword; here it is your {role}"));
		}
	});
}

/// Every name the program gives: parameters, variables, fields and functions, with their role
fn walk_names(node: &Node, found: &mut dyn FnMut(&str, &str)) {
	match node.drop_meta() {
		Node::Key(target, Op::Define | Op::Assign, body) => {
			match target.drop_meta() {
				Node::Symbol(name) => found(name, "variable"),
				Node::List(items, Bracket::Round, _) => {
					if let Some(name) = items.first().and_then(symbol_name) {
						found(&name, "function");
					}
					items.iter().skip(1).filter_map(parameter_name).for_each(|name| found(&name, "parameter"));
				}
				_ => {}
			}
			walk_names(body, found);
		}
		Node::Key(parameters, Op::FatArrow | Op::Arrow, body) => {
			let names: Vec<String> = match parameters.drop_meta() {
				Node::List(items, _, _) => items.iter().filter_map(parameter_name).collect(),
				other => parameter_name(other).into_iter().collect(),
			};
			names.iter().for_each(|name| found(name, "parameter"));
			walk_names(body, found);
		}
		Node::List(entries, Bracket::Curly, _) => entries.iter().for_each(|entry| {
			if let Node::Key(key, Op::Colon, _) = entry.drop_meta() {
				if let Some(name) = symbol_name(key) {
					found(&name, "field");
				}
			}
			walk_names(entry, found);
		}),
		Node::Key(left, _, right) => {
			walk_names(left, found);
			walk_names(right, found);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| walk_names(item, found)),
		_ => {}
	}
}

/// `x`, `x: int`, `x = 1`
fn parameter_name(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, Op::Colon | Op::Assign, _) => symbol_name(name),
		_ => None,
	}
}
