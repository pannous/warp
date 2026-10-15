//! Member completion (card member-completion): the names to offer after `receiver.` in the playground editor
//! (completion.js asks worker.js, which asks web_member_names) and in the console (cli/console.rs, on Tab).
//! The receiver is the operand right before the dot (`xs`, `[1 2]`, `"hi"`, `xs.sort()`), read in the light of the
//! statements before it: an instance's fields and methods (as `dir` lists them), a map's keys, a standard module's
//! words, else the methods of the receiver's type: the builtins below, then the words of the module named after the
//! type (lib/list.warp, text.warp, map.warp), which every program has.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::words::MAP_WORD;

/// The builtin methods of a list, text, map and number, as they are written after a dot, the most used first; each
/// works in the dot form (tests/control/test_member_completion.rs every_offered_builtin_is_a_method_of_its_type)
pub const LIST_MEMBERS: [&str; 28] = [
	"count", "sum", "map", "filter", "first", "last", "sort", "reverse", "join", "contains", "index_of", "min", "max",
	"mean", "reduce", "fold", "any", "all", "find", "each", "slice", "add", "insert", "remove", "pop", "clear", "copy",
	"norm",
];
pub const TEXT_MEMBERS: [&str; 18] = [
	"count", "upper", "lower", "trim", "split", "replace", "contains", "index_of", "first", "last", "slice", "reverse",
	"chars", "is_digit", "is_alpha", "number", "string", "copy",
];
pub const MAP_MEMBERS: [&str; 8] = ["keys", "values", "get", "count", "contains", "remove", "copy", "string"];
pub const NUMBER_MEMBERS: [&str; 10] =
	["string", "int", "float", "round", "floor", "ceil", "abs", "sqrt", "cbrt", "norm"];
const LIST_WORD: &str = "list";
const TEXT_WORD: &str = "text";
/// The variable the receiver is assigned to, so the passes that know variables (instance classes, map keys) know it
const RECEIVER: &str = "completion·receiver";
/// How many variables `ys = xs; zs = ys` is followed through to its value
const MOST_ALIASES: usize = 8;
const QUOTES: [char; 2] = ['"', '\''];

/// The names to offer after `code` and a dot, `code` ending with the receiver; empty when nothing is known of it
pub fn member_names(code: &str) -> Vec<String> {
	let (before, receiver) = split_receiver(code);
	if receiver.is_empty() {
		return vec![];
	}
	let parse = |text: &str| crate::normalize::without_hints(|| crate::warp_parser::parse(text));
	let receiver = parse(receiver);
	let program = parse(complete_statements(before));
	// a variable as itself, any other receiver as a variable it is assigned to, which the passes knowing variables know
	let (program, variable) = match receiver.symbol_name() {
		Some(name) => (program, name.to_string()),
		None => (with_assignment(program, receiver.clone()), RECEIVER.to_string()),
	};
	if let Some(names) = super::reflection::instance_names(&program, &variable) {
		return names;
	}
	let mut names =
		super::reflection::map_keys_of(&program, &variable).unwrap_or_else(|| module_names(&program, &receiver));
	names.extend(type_names(type_of(&receiver, &program, 0)));
	let mut seen = std::collections::HashSet::new();
	names.retain(|name| !name.contains('·') && seen.insert(name.clone()));
	names
}

fn with_assignment(program: Node, receiver: Node) -> Node {
	let assignment = super::nodes::key(Node::Symbol(RECEIVER.into()), Op::Assign, receiver);
	Node::List(vec![program, assignment], Bracket::None, Separator::Newline)
}

/// A standard module's words, unless the program names something so itself
fn module_names(program: &Node, receiver: &Node) -> Vec<String> {
	let module = receiver.symbol_name().filter(|name| !crate::library_words::defined_names(program).contains(*name));
	module.and_then(crate::modules::std_module_definitions).map(<[String]>::to_vec).unwrap_or_default()
}

/// The kinds of value whose type has methods to offer
#[derive(Clone, Copy)]
enum ValueType {
	List,
	Text,
	Map,
	Number,
}

/// Methods whose result has the receiver's type (`xs.sort().` offers a list's methods)
const TYPE_KEEPING_METHODS: [&str; 14] = [
	"sort", "sorted", "reverse", "reversed", "map", "filter", "slice", "copy", "unique", "take", "drop", "upper",
	"lower", "trim",
];

/// The methods of the value's type: the builtins, then the words of the module named after the type
fn type_names(value_type: Option<ValueType>) -> Vec<String> {
	let (builtins, module): (&[&str], &str) = match value_type {
		Some(ValueType::List) => (&LIST_MEMBERS, LIST_WORD),
		Some(ValueType::Text) => (&TEXT_MEMBERS, TEXT_WORD),
		Some(ValueType::Map) => (&MAP_MEMBERS, MAP_WORD),
		Some(ValueType::Number) => (&NUMBER_MEMBERS, ""),
		None => return vec![],
	};
	let module_words = crate::modules::std_module_definitions(module).unwrap_or_default();
	builtins.iter().map(|name| name.to_string()).chain(module_words.iter().cloned()).collect()
}

/// The type of a value written in the program: a variable's is its last assigned value's (followed through other
/// variables), a method call's the method's result's
fn type_of(node: &Node, program: &Node, depth: usize) -> Option<ValueType> {
	if depth > MOST_ALIASES {
		return None;
	}
	match node.drop_meta() {
		Node::Symbol(name) => type_of(&assigned_value(program, name)?, program, depth + 1),
		Node::List(_, Bracket::Curly, _) => Some(ValueType::Map),
		Node::List(items, Bracket::None, _) if items.len() == 1 => type_of(&items[0], program, depth),
		Node::Key(receiver, Op::Dot, call) => {
			let method = match call.drop_meta() {
				Node::List(items, _, _) => items.first()?.symbol_name()?.to_string(),
				method => method.symbol_name()?.to_string(),
			};
			match crate::library_words::result_kind(&method) {
				Some(kind) => kind_type(kind),
				None if TYPE_KEEPING_METHODS.contains(&method.as_str()) => type_of(receiver, program, depth + 1),
				None => None,
			}
		}
		value => kind_type(crate::analyzer::infer_type(value, &crate::analyzer::Scope::new())),
	}
}

fn kind_type(kind: Kind) -> Option<ValueType> {
	match kind {
		Kind::List => Some(ValueType::List),
		Kind::Text | Kind::Codepoint => Some(ValueType::Text),
		Kind::Int | Kind::Float => Some(ValueType::Number),
		_ => None,
	}
}

fn assigned_value(program: &Node, name: &str) -> Option<Node> {
	let mut last = None;
	program.visit(&mut |part| {
		if let Node::Key(target, Op::Assign | Op::Define, value) = part {
			if target.symbol_name() == Some(name) {
				last = Some(value.as_ref().clone());
			}
		}
	});
	last
}

/// `xs = [1 2]; print xs` → ("xs = [1 2]; print ", "xs"): the operand ending the code, a dotted chain or call included
fn split_receiver(code: &str) -> (&str, &str) {
	let code = code.trim_end();
	let mut start = code.len();
	loop {
		let atom = atom_start(&code[..start]);
		if atom == start {
			break;
		}
		start = atom;
		let before = &code[..start];
		if before.ends_with('.') && atom_start(&before[..before.len() - 1]) < before.len() - 1 {
			start -= 1;
		} else if !(code[start..].starts_with('(') && before.ends_with(is_word_character)) {
			break;
		}
	}
	(&code[..start], &code[start..])
}

/// Where the last name, number, quoted text or bracketed group of `code` starts; its length when there is none
fn atom_start(code: &str) -> usize {
	let Some(last) = code.chars().next_back() else { return 0 };
	if is_word_character(last) {
		return code.trim_end_matches(is_word_character).len();
	}
	if QUOTES.contains(&last) {
		return code[..code.len() - 1].rfind(last).unwrap_or(code.len());
	}
	let Some(opener) = opening_bracket(last) else { return code.len() };
	let mut depth = 0;
	for (at, character) in code.char_indices().rev() {
		if character == last {
			depth += 1;
		} else if character == opener {
			depth -= 1;
			if depth == 0 {
				return at;
			}
		}
	}
	code.len()
}

fn opening_bracket(closing: char) -> Option<char> {
	match closing {
		')' => Some('('),
		']' => Some('['),
		'}' => Some('{'),
		_ => None,
	}
}

fn is_word_character(character: char) -> bool {
	character.is_alphanumeric() || character == '_'
}

/// The code up to its last statement boundary: what a half-written statement before the receiver (`print `) leaves
fn complete_statements(code: &str) -> &str {
	let (mut depth, mut quote, mut end) = (0i32, None, 0);
	for (at, character) in code.char_indices() {
		match (quote, character) {
			(Some(open), _) if character == open => quote = None,
			(Some(_), _) => {}
			(None, _) if QUOTES.contains(&character) => quote = Some(character),
			(None, '(' | '[' | '{') => depth += 1,
			(None, ')' | ']' | '}') => depth -= 1,
			(None, ';' | '\n') if depth == 0 => end = at,
			_ => {}
		}
	}
	&code[..end]
}
