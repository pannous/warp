//! Matching by type name (D5, notes/matching.md): known type words in a definition head name typed parameters.
//! `fib int i = …` → `fib(i:int) := …`, `fibonacci number = …` → `fibonacci(number:number) := …`,
//! `foo of int = it+it` → `foo(it:int) := …`; the `to` phrase `to square a number:` shares `parameter_slots`.

use super::nodes::{call, is_block, is_call_head, key, spaced_statement};
use crate::analyzer::type_word_kind;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;

const ARTICLES: [&str; 3] = ["a", "an", "the"];
/// The words that separate a phrase's slots; `each` too: `to hum notes each d: …` is called `hum [C4 E4] each 50ms`
pub(crate) const PREPOSITIONS: [&str; 11] = ["to", "of", "from", "with", "in", "into", "at", "by", "for", "on", "each"];
const IT: &str = "it";
/// Statement words before a typed declaration that never name a function: `global number = 3`, `let int x = 1`
const STATEMENT_WORDS: [&str; 20] = [
	"global", "let", "var", "export", "mutable", "mut", "return", "yield", "print", "println", "puts", "not", "use", "import",
	"include", "if", "while", "for", "else", "then",
];

pub(crate) fn names_a_function(name: &str) -> bool {
	!is_type_word(name) && !ARTICLES.contains(&name) && !PREPOSITIONS.contains(&name) && !STATEMENT_WORDS.contains(&name)
		&& !crate::analyzer::CONSTANT_KEYWORDS.contains(&name) && !crate::operators::FUNCTION_KEYWORDS.contains(&name)
}

fn is_type_word(word: &str) -> bool {
	type_word_kind(word).is_some()
}

fn typed(name: &str, type_name: &str) -> Node {
	key(symbol(name), Op::Colon, symbol(type_name))
}

/// Which head words are prepositions
pub fn prepositions_among(words: &[&str]) -> Vec<bool> {
	words.iter().map(|word| PREPOSITIONS.contains(word)).collect()
}

/// Which head words belong to the phrase rather than name parameters (card phrase-multiword): the prepositions and,
/// in a slot after one, the plain words before the slot's last that the body never uses: `with subject title` is
/// called `with subject "hi"`
pub fn phrase_words(words: &[&str], body: &Node, is_known_type: &dyn Fn(&str) -> bool) -> Vec<bool> {
	let mut in_phrase = prepositions_among(words);
	for index in 1..words.len() {
		let word = words[index];
		let more_in_slot = words.get(index + 1).is_some_and(|next| !PREPOSITIONS.contains(next));
		in_phrase[index] |= in_phrase[index - 1] && more_in_slot && !ARTICLES.contains(&word) && !is_known_type(word) && !uses_name(body, word);
	}
	in_phrase
}

/// The parameters of the head words after the function name (notes/matching.md):
/// - in a slot after a preposition, plain words the body never uses belong to the phrase (phrase_words)
/// - prepositions (`to of from …`) only separate slots: `to add number a to number b` → `a:number`, `b:number`
/// - an article starts a noun slot when a known type follows it or the body never uses the article as a name:
///   `a number` → `number:number`, `a photo` → `photo` (`photo:photo` once `class photo` is declared); `to add a b: a+b` keeps `a`
/// - a known type before a name types it: `int i` → `i:int`; a lone known type names itself, or is `it` when the body uses `it`
/// - several nouns after an article are one multi-word name, typed by its head noun and named by it: `a phone number` → `number:number`
///
/// - a type word repeated alone numbers its parameters: `combine float with float = float#1 + float#2` →
///   `combine(float·1:float, float·2:float) := float·1 + float·2`, the body read the same way
///
/// Err when two parameters end up with one name (`a first name`, `a last name`).
pub fn parameter_slots(words: &[&str], body: &Node, is_known_type: &dyn Fn(&str) -> bool) -> Result<(Vec<Node>, Node), String> {
	let in_phrase = phrase_words(words, body, is_known_type);
	let is_name = |word: &str| !PREPOSITIONS.contains(&word) && !is_known_type(word);
	let mut parameters = vec![];
	// `number a to …`: an article right before a preposition or the end is a name
	let ends_slot = |index: usize| in_phrase.get(index).is_none_or(|phrase_word| *phrase_word);
	let mut index = 0;
	while index < words.len() {
		let word = words[index];
		let next = (!ends_slot(index + 1)).then(|| words[index + 1]);
		if in_phrase[index] {
			index += 1;
		} else if ARTICLES.contains(&word) && next.is_some_and(|next| is_known_type(next) || !uses_name(body, word)) {
			let noun: Vec<&str> = (index + 1..words.len()).take_while(|later| !ends_slot(*later) && !ARTICLES.contains(&words[*later])).map(|later| words[later]).collect();
			let head = noun[noun.len() - 1];
			parameters.push(if is_known_type(head) { typed(head, head) } else { symbol(head) });
			index += 1 + noun.len();
		} else if is_known_type(word) && next.is_some_and(|next| is_name(next) && (!ARTICLES.contains(&next) || ends_slot(index + 2))) {
			parameters.push(typed(next.unwrap_or_default(), word));
			index += 2;
		} else {
			parameters.push(if is_known_type(word) { typed(word, word) } else { symbol(word) });
			index += 1;
		}
	}
	let mut parameters = numbered_repeated_types(parameters);
	let body = numbered_type_reads(body, &parameters);
	let names: Vec<String> = parameters.iter().map(Node::name).collect();
	if let Some(shared) = names.iter().enumerate().find(|(index, name)| names[..*index].contains(name)).map(|(_, name)| name) {
		return Err(format!("two parameters are named {shared}: use one-word parameter names here (`first name`, `last name` need multi-word identifiers)"));
	}
	if let [Node::Key(name, Op::Colon, type_name)] = parameters.as_slice() {
		// `fibonacci number := … number … it …` uses both: the parameter keeps its name and `it` is the same value
		if name == type_name && uses_it(&body) && !uses_name(&body, &name.name()) {
			parameters = vec![typed(IT, &type_name.name())];
		}
	}
	Ok((parameters, body))
}

const NUMBER_JOINER: &str = "·";

/// The lone type parameters `float:float` that repeat, as `float·1:float`, `float·2:float`
fn numbered_repeated_types(parameters: Vec<Node>) -> Vec<Node> {
	let lone_type = |parameter: &Node| match parameter.drop_meta() {
		Node::Key(name, Op::Colon, type_name) if name == type_name => Some(type_name.name()),
		_ => None,
	};
	let repeated = |type_name: &str| parameters.iter().filter(|parameter| lone_type(parameter).as_deref() == Some(type_name)).count() > 1;
	let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
	parameters.iter().map(|parameter| match lone_type(parameter).filter(|type_name| repeated(type_name)) {
		Some(type_name) => {
			let number = seen.entry(type_name.clone()).or_default();
			*number += 1;
			typed(&format!("{type_name}{NUMBER_JOINER}{number}"), &type_name)
		}
		None => parameter.clone(),
	}).collect()
}

/// `float#2` of the body as the numbered parameter `float·2` it names
fn numbered_type_reads(body: &Node, parameters: &[Node]) -> Node {
	let names: Vec<String> = parameters.iter().map(Node::name).filter(|name| name.contains(NUMBER_JOINER)).collect();
	if names.is_empty() {
		return body.clone();
	}
	fn rewrite(node: &Node, names: &[String]) -> Node {
		if let Node::Key(type_name, Op::Hash, number) = node.drop_meta() {
			if let (Node::Symbol(type_name), Node::Number(number)) = (type_name.drop_meta(), number.drop_meta()) {
				let numbered = format!("{type_name}{NUMBER_JOINER}{number}");
				if names.contains(&numbered) {
					return Node::Symbol(numbered);
				}
			}
		}
		node.clone().map_children(|child| rewrite(&child, names))
	}
	rewrite(body, &names)
}

fn uses_name(node: &Node, wanted: &str) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(name) if name == wanted));
	found
}

pub fn uses_it(node: &Node) -> bool {
	uses_name(node, IT)
}

/// The parameters of a spaced definition head `name words… = body` (also `:=`): its slots when a word is a known type
/// word; None for `f x = …` without one, which keeps its old meaning (each word a parameter)
pub fn spaced_parameters(words: &[&str], body: &Node) -> Option<Result<(Vec<Node>, Node), String>> {
	words.iter().any(|word| is_type_word(word)).then(|| parameter_slots(words, body, &is_type_word))
}

/// The statement `name words… last = body` (parsed as the items `name`, `words…`, `last=body`) as `name(params) := body`
fn spaced_definition(items: &[Node]) -> Option<Node> {
	let (name, words, body) = spaced_statement(items)?;
	let name = name.symbol_name().filter(|name| names_a_function(name))?;
	let (parameters, body) = match spaced_parameters(&words, body)? {
		Ok(slots) => slots,
		Err(message) => return Some(crate::node::error(&message)),
	};
	let head = call(name, parameters);
	Some(key(head, Op::Define, body))
}

/// A function head `name(params)`, as a call
/// A definition with a declared return type, as the definition whose body converts its result to that type:
/// `int square(x) = x*x` (items `int`, `square(x) = …`), `square(x) as int := …` and `square(x) : int = …` (also after
/// `def`) → `square(x) := (x*x) as int`
fn typed_return(type_name: &Node, definition: &Node) -> Option<Node> {
	// `function square(n){…}` names no return type: a definition keyword is no type word here
	type_name.symbol_name().filter(|word| is_type_word(word) && !crate::operators::FUNCTION_KEYWORDS.contains(word))?;
	let type_name = type_name.drop_meta();
	match definition.drop_meta() {
		Node::Key(head, op @ (Op::Assign | Op::Define), body) if is_call_head(head) => {
			Some(Node::Key(head.clone(), *op, Box::new(converted(body, type_name))))
		}
		// `int half(x){ … }`: the signature glued to its block. Without the type word `half(x){…}` stays no definition
		// (`if(c){…}` has the same shape); with it the definition is unambiguous: `half(x) := {…}`
		Node::List(items, Bracket::Round, _) if matches!(items.as_slice(), [head, body] if is_call_head(head) && is_block(body)) => {
			Some(key(flat_head(&items[0]), Op::Define, converted(&items[1], type_name)))
		}
		_ => None,
	}
}

/// The glued signature keeps its parameters as one group, `(half (x))`: `(half x)` like the head of `half(x) := …`
fn flat_head(head: &Node) -> Node {
	match head.drop_meta() {
		Node::List(items, bracket, separator) if items.len() == 2 => match items[1].drop_meta() {
			Node::List(parameters, _, _) => Node::List([vec![items[0].clone()], parameters.clone()].concat(), bracket.clone(), separator.clone()),
			_ => head.clone(),
		},
		_ => head.clone(),
	}
}


/// The value converted to the declared result type; an arithmetic value is grouped, `(x*2) as int`, as written it
/// would be the ambiguous `x*2 as int`
fn as_type(value: Node, type_name: &Node) -> Node {
	let value = match value.drop_meta() {
		Node::Key(_, op, _) if op.is_arithmetic() => Node::List(vec![value], Bracket::Round, Separator::None),
		_ => value,
	};
	key(value, Op::As, type_name.clone())
}

const RETURN: &str = "return";

fn is_return(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if items.len() == 2 && items[0].symbol_name() == Some(RETURN))
}

/// The body with every value it gives back converted: each `return e` and the last statement of a block
fn converted(body: &Node, type_name: &Node) -> Node {
	let body = converted_returns(body.drop_meta().clone(), type_name);
	match body {
		Node::List(mut items, Bracket::Curly, separator) if !items.is_empty() => {
			let last = items.pop().expect("not empty");
			items.push(if is_return(&last) { last } else { as_type(last, type_name) });
			Node::List(items, Bracket::Curly, separator)
		}
		body if is_return(&body) => body,
		body => as_type(body, type_name),
	}
}

fn converted_returns(node: Node, type_name: &Node) -> Node {
	match node {
		Node::List(mut items, bracket, separator) if items.len() == 2 && items[0].symbol_name() == Some(RETURN) => {
			let value = items.pop().expect("two items");
			items.push(as_type(value, type_name));
			Node::List(items, bracket, separator)
		}
		other => other.map_children(|child| converted_returns(child, type_name)),
	}
}

/// `square(x) as int := …` / `square(x) : int = …`: the head with its return type, as `typed_return`
fn return_typed_definition(node: &Node) -> Option<Node> {
	let Node::Key(target, op @ (Op::Assign | Op::Define), body) = node.drop_meta() else { return None };
	let Node::Key(head, Op::As | Op::Colon, type_name) = target.drop_meta() else { return None };
	typed_return(type_name, &Node::Key(head.clone(), *op, body.clone()))
}

pub fn lower(node: Node) -> Node {
	if let Some(definition) = return_typed_definition(&node) {
		return definition;
	}
	match node {
		Node::List(items, Bracket::None, Separator::Space) if matches!(items.as_slice(), [type_name, definition] if typed_return(type_name, definition).is_some()) => {
			typed_return(&items[0], &items[1]).expect("guarded")
		}
		Node::List(items, Bracket::None, Separator::Space) if spaced_definition(&items).is_some() => {
			spaced_definition(&items).expect("guarded")
		}
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		other => other,
	}
}
