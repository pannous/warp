//! Keywords (P165, user): hard keywords — control flow, declarations, literal values, modules — are never redefined.
//! Every other keyword-like word is soft: it may name a local variable, parameter, field or method, where the program's
//! name wins with a got-it note ("send is a keyword; here it is your variable"); redefining one at the top level is a
//! loud error. Passes that give a soft keyword its meaning ask `program_names` whether the program took the word.

use super::nodes::{parameter_name, word_headed};
use crate::errors::diagnostic::Diagnostic;
use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::type_name_matching::is_preposition;

pub const HARD_KEYWORDS: [&str; 27] = [
	"if", "then", "else", "while", "for", "in", "return", "break", "continue", // control flow
	"def", "fun", "fn", "class", "var", "let", "const", "val", "global", // declarations
	"true", "false", "ø", "null", "nil", // literal values
	"use", "import", "include", // modules
	"data",    // quoted code (user, card data-reserved)
];
/// The hard keywords refused also as a parameter or loop variable; the others head arms and lambdas there
/// (`else -> 20`, PHP's `fn($x) => …`)
const UNBOUND_KEYWORDS: [&str; 1] = ["data"];
/// emit's aliases fire, trigger and signal stay out: a program may define them (P163, test_event_footguns; asked)
pub const SOFT_KEYWORDS: [&str; 12] =
	["emit", "send", "broadcast", "every", "whenever", "init", "new", "root", "listeners", "stored", "undo", "redo"];
/// Keyword-like words in neither list: the playground editor colors them like soft keywords (web/playground/keywords.py
/// makes keywords.js; it also colors every `const …_KEYWORD: &str` in src/, so a pass registers its word by that name). Display only: P165 gives them no meaning; moving one into a list above is a user decision
pub const HIGHLIGHTED_WORDS: [&str; 63] = [
	"try",
	"catch",
	"except",
	"switch",
	"match",
	"case",
	"when",
	"do",
	"yield",
	"await",
	"go",
	"elif",
	"elsif",
	"elseif",
	"end",
	"until",
	"law",
	"on",
	"once",
	"within",
	"after",
	"of",
	"from",
	"to",
	"upto",
	"times",
	"is",
	"as",
	"with",
	"extends",
	"mixin",
	"static",
	"trait",
	"interface",
	"protocol",
	"struct",
	"record",
	"operator",
	"nonlocal",
	"lambda",
	"define",
	"function",
	"func",
	"result",
	"print",
	"assert",
	"linear",
	"shared",
	"ref",
	"meta",
	"raise",
	"throw",
	"fail",
	"panic",
	"warn",
	"require",
	"and",
	"or",
	"not",
	"xor",
	"fire",
	"trigger",
	"signal",
];
/// Keyword-like words that stand for a value: a word operator after them takes them as its left operand, `result plus 1`
const VALUE_WORDS: [&str; 6] = ["true", "false", "ø", "null", "nil", "result"];

/// A word of the keyword lists that starts or joins a statement and is no value of its own: `do`, `return`, `on`, `emit`
pub fn is_statement_word(word: &str) -> bool {
	HARD_KEYWORDS.iter().chain(&SOFT_KEYWORDS).chain(&HIGHLIGHTED_WORDS).any(|keyword| *keyword == word)
		&& !VALUE_WORDS.contains(&word)
}

const TOPIC: &str = "soft-keyword";

pub fn lower(program: Node) -> Node {
	if let Some(error) = first_refused(&program, true) {
		return error;
	}
	note_local_names(&program);
	program
}

/// A hard keyword assigned anywhere, a soft one defined at the top level: the loud error
fn first_refused(node: &Node, top_level: bool) -> Option<Node> {
	if let Some(name) = bound_names(node).into_iter().find(|name| UNBOUND_KEYWORDS.contains(&name.as_str())) {
		return Some(Diagnostic::at(node, format!("{name} is a keyword: give it another name")).into_error());
	}
	if let Some(name) = node.defined_name() {
		let refused = HARD_KEYWORDS.contains(&name.as_str()) || top_level && SOFT_KEYWORDS.contains(&name.as_str());
		if refused {
			let fix = if top_level && SOFT_KEYWORDS.contains(&name.as_str()) {
				"give it another name; a local variable, parameter or field may use it"
			} else {
				"give it another name"
			};
			return Some(Diagnostic::at(node, format!("{name} is a keyword: {fix}")).into_error());
		}
	}
	match node.drop_meta() {
		// `play 440Hz for=0.5s`: the argument named by its preposition (preposition_calls.rs), only its value is checked
		Node::List(items, _, _) if preposition_arguments(items) => {
			items.iter().find_map(|item| match item.drop_meta() {
				Node::Key(name, Op::Assign, value) if is_preposition(name) => first_refused(value, false),
				_ => first_refused(item, false),
			})
		}
		// the statements of the program, or of a block at its top level, stay at the top level
		Node::List(items, Bracket::None | Bracket::Curly, _) if top_level => {
			items.iter().find_map(|item| first_refused(item, true))
		}
		Node::Key(_, Op::Define | Op::Assign, body) if node.defined_name().is_some() => first_refused(body, false),
		Node::Key(left, _, right) => first_refused(left, false).or_else(|| first_refused(right, false)),
		Node::List(items, _, _) => items.iter().find_map(|item| first_refused(item, false)),
		_ => None,
	}
}

/// The parameters a definition or lambda binds, the variable of a `for … in` loop
fn bound_names(node: &Node) -> Vec<String> {
	match node.drop_meta() {
		Node::Key(head, Op::Define | Op::Assign, _) => match head.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.iter().skip(1).filter_map(parameter_name).collect(),
			_ => vec![],
		},
		Node::Key(parameters, Op::Arrow, _) => match parameters.drop_meta() {
			Node::List(items, _, _) => items.iter().filter_map(parameter_name).collect(),
			other => parameter_name(other).into_iter().collect(),
		},
		Node::List(words, _, _) => super::nodes::for_in_variable(words).into_iter().collect(),
		_ => vec![],
	}
}

/// A call `f a for=b` or `f(a, for=b)`: a function's name, then an argument named by a preposition
fn preposition_arguments(items: &[Node]) -> bool {
	word_headed(items).is_some()
		&& items[1..]
			.iter()
			.any(|item| matches!(item.drop_meta(), Node::Key(name, Op::Assign, _) if is_preposition(name)))
}

/// The got-it note for each soft keyword the program uses as a local name
fn note_local_names(program: &Node) {
	for (name, meaning) in crate::program_names::Bindings::of(program).meanings() {
		if SOFT_KEYWORDS.contains(&name) {
			crate::errors::diagnostic::educate_once(
				&format!("{TOPIC}-{name}"),
				name,
				name,
				&format!("{name} is a keyword; here it is your {}", meaning.word()),
			);
		}
	}
}
