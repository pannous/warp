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

/// The parameters of the head words after the function name, when a known type word makes them typed slots:
/// `int i` → `i:int`, `a number` → `number:number`, `of int` → `int:int`; a lone type word is `it` when the body uses `it`.
/// None when no type word is among them (plain names keep their old meaning).
pub fn parameter_slots(words: &[&str], body_uses_it: bool) -> Option<Vec<Node>> {
	if !words.iter().any(|word| is_type_word(word)) {
		return None;
	}
	let mut parameters = vec![];
	let mut rest = words.iter().copied().peekable();
	while let Some(word) = rest.next() {
		let next = rest.peek().copied();
		match next {
			_ if PREPOSITIONS.contains(&word) => {}
			Some(_) if ARTICLES.contains(&word) => {}
			Some(name) if is_type_word(word) && !is_type_word(name) && !PREPOSITIONS.contains(&name) && !ARTICLES.contains(&name) => {
				parameters.push(typed(name, word));
				rest.next();
			}
			_ => parameters.push(Node::Symbol(word.to_string())),
		}
	}
	if let [Node::Symbol(lone)] = parameters.as_slice() {
		let lone = lone.clone();
		parameters = vec![typed(if body_uses_it { IT } else { &lone }, &lone)];
	}
	if parameters.len() > 1 {
		parameters = parameters.into_iter().map(|parameter| match word(&parameter) {
			Some(name) if is_type_word(name) => typed(name, name),
			_ => parameter,
		}).collect();
	}
	Some(parameters)
}

pub fn uses_it(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if name == IT));
	found
}

/// The statement `name words… last = body` (parsed as the items `name`, `words…`, `last=body`) as `name(params) := body`
fn spaced_definition(items: &[Node]) -> Option<Node> {
	let (name, rest) = items.split_first()?;
	let (last, middle) = rest.split_last()?;
	let name = word(name).filter(|name| names_a_function(name))?;
	let Node::Key(last_word, Op::Assign | Op::Define, body) = last.drop_meta() else { return None };
	let words: Vec<&str> = middle.iter().chain(std::iter::once(last_word.as_ref())).map(word).collect::<Option<_>>()?;
	let parameters = parameter_slots(&words, uses_it(body))?;
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
