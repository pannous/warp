//! Matching by type name (D5, notes/matching.md): known type words in a definition head name typed parameters.
//! `fib int i = …` → `fib(i:int) := …`, `fibonacci number = …` → `fibonacci(number:number) := …`,
//! `foo of int = it+it` → `foo(it:int) := …`; the `to` phrase `to square a number:` shares `parameter_slots`.

use crate::analyzer::type_word_kind;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const ARTICLES: [&str; 3] = ["a", "an", "the"];
const PREPOSITIONS: [&str; 10] = ["to", "of", "from", "with", "in", "into", "at", "by", "for", "on"];
const IT: &str = "it";
/// Statement words before a typed declaration that never name a function: `global number = 3`, `let int x = 1`
const STATEMENT_WORDS: [&str; 20] = [
	"global", "let", "var", "export", "mutable", "mut", "return", "yield", "print", "println", "puts", "not", "use", "import",
	"include", "if", "while", "for", "else", "then",
];

fn names_a_function(name: &str) -> bool {
	!is_type_word(name) && !ARTICLES.contains(&name) && !PREPOSITIONS.contains(&name) && !STATEMENT_WORDS.contains(&name)
		&& !crate::analyzer::CONSTANT_KEYWORDS.contains(&name) && !crate::operators::FUNCTION_KEYWORDS.contains(&name)
}

fn word(node: &Node) -> Option<&str> {
	match node.drop_meta() {
		Node::Symbol(name) => Some(name),
		_ => None,
	}
}

fn is_type_word(word: &str) -> bool {
	type_word_kind(word).is_some()
}

fn typed(name: &str, type_name: &str) -> Node {
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Colon, Box::new(Node::Symbol(type_name.to_string())))
}

/// The parameters of the head words after the function name (notes/matching.md):
/// - prepositions (`to of from …`) only separate slots: `to add number a to number b` → `a:number`, `b:number`
/// - an article starts a noun slot when a known type follows it or the body never uses the article as a name:
///   `a number` → `number:number`, `a photo` → `photo` (`photo:photo` once `class photo` is declared); `to add a b: a+b` keeps `a`
/// - a known type before a name types it: `int i` → `i:int`; a lone known type names itself, or is `it` when the body uses `it`
/// - several nouns after an article are one multi-word name, typed by its head noun and named by it: `a phone number` → `number:number`
/// Err when two parameters end up with one name (`a first name`, `a last name`).
pub fn parameter_slots(words: &[&str], body: &Node, is_known_type: &dyn Fn(&str) -> bool) -> Result<Vec<Node>, String> {
	let is_name = |word: &str| !PREPOSITIONS.contains(&word) && !is_known_type(word);
	let mut parameters = vec![];
	// `number a to …`: an article right before a preposition or the end is a name
	let ends_slot = |index: usize| words.get(index).is_none_or(|word| PREPOSITIONS.contains(word));
	let mut index = 0;
	while index < words.len() {
		let word = words[index];
		let next = words.get(index + 1).copied().filter(|next| !PREPOSITIONS.contains(next));
		if PREPOSITIONS.contains(&word) {
			index += 1;
		} else if ARTICLES.contains(&word) && next.is_some_and(|next| is_known_type(next) || !uses_name(body, word)) {
			let noun: Vec<&str> = words[index + 1..].iter().copied().take_while(|word| !PREPOSITIONS.contains(word) && !ARTICLES.contains(word)).collect();
			let head = noun[noun.len() - 1];
			parameters.push(if is_known_type(head) { typed(head, head) } else { Node::Symbol(head.to_string()) });
			index += 1 + noun.len();
		} else if is_known_type(word) && next.is_some_and(|next| is_name(next) && (!ARTICLES.contains(&next) || ends_slot(index + 2))) {
			parameters.push(typed(next.unwrap_or_default(), word));
			index += 2;
		} else {
			parameters.push(if is_known_type(word) { typed(word, word) } else { Node::Symbol(word.to_string()) });
			index += 1;
		}
	}
	let names: Vec<String> = parameters.iter().map(Node::name).collect();
	if let Some(shared) = names.iter().enumerate().find(|(index, name)| names[..*index].contains(name)).map(|(_, name)| name) {
		return Err(format!("two parameters are named {shared}: use one-word parameter names here (`first name`, `last name` need multi-word identifiers)"));
	}
	if let [Node::Key(name, Op::Colon, type_name)] = parameters.as_slice() {
		if name == type_name && uses_it(body) {
			parameters = vec![typed(IT, &type_name.name())];
		}
	}
	Ok(parameters)
}

fn uses_name(node: &Node, wanted: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if name == wanted));
	found
}

pub fn uses_it(node: &Node) -> bool {
	uses_name(node, IT)
}

/// The statement `name words… last = body` (parsed as the items `name`, `words…`, `last=body`) as `name(params) := body`
fn spaced_definition(items: &[Node]) -> Option<Node> {
	let (name, rest) = items.split_first()?;
	let (last, middle) = rest.split_last()?;
	let name = word(name).filter(|name| names_a_function(name))?;
	let Node::Key(last_word, Op::Assign | Op::Define, body) = last.drop_meta() else { return None };
	let words: Vec<&str> = middle.iter().chain(std::iter::once(last_word.as_ref())).map(word).collect::<Option<_>>()?;
	if !words.iter().any(|word| is_type_word(word)) {
		return None; // `f x = …` without a type word keeps its old meaning
	}
	let parameters = match parameter_slots(&words, body, &is_type_word) {
		Ok(parameters) => parameters,
		Err(message) => return Some(crate::node::error(&message)),
	};
	let head = Node::List([vec![Node::Symbol(name.to_string())], parameters].concat(), Bracket::Round, Separator::None);
	Some(Node::Key(Box::new(head), Op::Define, body.clone()))
}

pub fn lower(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::None, Separator::Space) if spaced_definition(&items).is_some() => {
			spaced_definition(&items).expect("guarded")
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}
