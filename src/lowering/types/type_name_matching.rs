//! Matching by type name (D5, notes/matching.md): known type words in a definition head name typed parameters.
//! `fibonacci number = …` → `fibonacci(number:number) := …`,
//! `foo of int = it+it` → `foo(it:int) := …`; the `to` phrase `to square a number:` shares `parameter_slots`.

use super::nodes::{call, is_block, is_call_head, key, spaced_statement};
use super::nodes::{is_return, typed};
use crate::analyzer::type_word_kind;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{ANY_TYPE, ERROR_WORD, IMPLICIT_PARAMETER};

const ARTICLES: [&str; 3] = ["a", "an", "the"];
/// The words that separate a phrase's slots and name the slot after them (preposition_calls.rs); `each` too: `to hum notes each d: …` is called `hum [C4 E4] each 50ms`
pub(crate) const PREPOSITIONS: [&str; 11] = ["to", "of", "from", "with", "in", "into", "at", "by", "for", "on", "each"];

/// `for`, `to`, …: a symbol that is a preposition
pub(crate) fn is_preposition(word: &Node) -> bool {
	matches!(word.drop_meta(), Node::Symbol(name) if PREPOSITIONS.contains(&name.as_str()))
}

/// `as` labels a parameter too, though it separates no phrase slot: `save(made, as file)` is called
/// `save sound as "song.wav"` (card save-sound)
pub(crate) const AS_LABEL: &str = "as";

/// A word naming the parameter after it: a preposition or `as`
pub(crate) fn is_label_word(word: &Node) -> bool {
	is_preposition(word) || word.drop_meta().is_symbol(AS_LABEL)
}

/// Statement words before a typed declaration that never name a function: `global number = 3`, `let int x = 1`
const STATEMENT_WORDS: [&str; 17] = [
	"global", "let", "var", "export", "mutable", "mut", "return", "yield", "print", "println", "puts", "not", "if",
	"while", "for", "else", "then",
];

pub(crate) fn names_a_function(name: &str) -> bool {
	!is_kinded_type_or_any(name)
		&& !ARTICLES.contains(&name)
		&& !PREPOSITIONS.contains(&name)
		&& !STATEMENT_WORDS.contains(&name)
		&& !crate::modules::is_import_keyword(name)
		&& !crate::words::CONSTANT_KEYWORDS.contains(&name)
		&& !crate::operators::FUNCTION_KEYWORDS.contains(&name)
}

/// A type word of a head: a type with a value kind (analyzer::type_word_kind) or `any`: `fib int i`, `foo of any`
fn is_kinded_type_or_any(word: &str) -> bool {
	type_word_kind(word).is_some() || word == crate::words::ANY_TYPE
}

/// Which head words are prepositions
fn prepositions_among(words: &[&str]) -> Vec<bool> {
	words.iter().map(|word| PREPOSITIONS.contains(word)).collect()
}

/// The parameters of the head words after the function name (notes/matching.md):
/// - prepositions (`to of from …`) only separate slots: `to add number a to number b` → `a:number`, `b:number`
/// - an article starts a noun slot when a known type follows it or the body never uses the article as a name:
///   `a number` → `number:number`, `a photo` → `photo` (`photo:photo` once `class photo` is declared); `to add a b: a+b` keeps `a`
/// - a lone known type names itself, or is `it` when the body uses `it`; a type before a name (`fib int i`) is no
///   longer read (decision cleanup-cuts): write `fib(i:int)`
/// - several nouns after an article are one multi-word name, typed by its head noun and named by it: `a phone number` → `number:number`
///
/// Err when two parameters end up with one name (`a first name`, `a last name`).
pub fn parameter_slots(
	words: &[&str],
	body: &Node,
	is_known_type: &dyn Fn(&str) -> bool,
	labelled: bool,
) -> Result<(Vec<Node>, Node), String> {
	let in_phrase = prepositions_among(words);
	let mut parameters = vec![];
	// `from a`: the preposition before the slot, which names it in calls when `labelled` (`move 1 from 2`)
	let mut preposition = None;
	let mut push = |parameter: Node, preposition: &mut Option<&str>| match preposition.take().filter(|_| labelled) {
		Some(word) => parameters.push(Node::List(vec![symbol(word), parameter], Bracket::None, Separator::Space)),
		None => parameters.push(parameter),
	};
	// `number a to …`: an article right before a preposition or the end is a name
	let ends_slot = |index: usize| in_phrase.get(index).is_none_or(|phrase_word| *phrase_word);
	let mut index = 0;
	while index < words.len() {
		let word = words[index];
		let next = (!ends_slot(index + 1)).then(|| words[index + 1]);
		if in_phrase[index] {
			preposition = Some(word);
			index += 1;
		} else if ARTICLES.contains(&word) && next.is_some_and(|next| is_known_type(next) || !body.mentions(word)) {
			let noun: Vec<&str> = (index + 1..words.len())
				.take_while(|later| !ends_slot(*later) && !ARTICLES.contains(&words[*later]))
				.map(|later| words[later])
				.collect();
			let head = noun[noun.len() - 1];
			push(if is_known_type(head) { typed(head, head) } else { symbol(head) }, &mut preposition);
			index += 1 + noun.len();
		} else {
			push(if is_known_type(word) { typed(word, word) } else { symbol(word) }, &mut preposition);
			index += 1;
		}
	}
	let body = body.clone();
	let names: Vec<String> = parameters.iter().map(crate::lowering::nodes::parameter_word).collect();
	if let Some(shared) =
		names.iter().enumerate().find(|(index, name)| names[..*index].contains(name)).map(|(_, name)| name)
	{
		return Err(format!(
			"two parameters are named {shared}: use one-word parameter names here (`first name`, `last name` need multi-word identifiers)"
		));
	}
	if let [Node::Key(name, Op::Colon, type_name)] = parameters.as_slice() {
		// `fibonacci number := … number … it …` uses both: the parameter keeps its name and `it` is the same value
		if name == type_name && crate::analyzer::uses_it(&body) && !body.mentions(&name.name()) {
			parameters = vec![typed(IMPLICIT_PARAMETER, &type_name.name())];
		}
	}
	Ok((parameters, body))
}

/// The parameters of a spaced definition head `name words… = body` (also `:=`): its slots when a word is a known type
/// word; None for `f x = …` without one, which keeps its old meaning (each word a parameter)
pub fn spaced_parameters(words: &[&str], body: &Node) -> Option<Result<(Vec<Node>, Node), String>> {
	words
		.iter()
		.any(|word| is_kinded_type_or_any(word))
		.then(|| parameter_slots(words, body, &is_kinded_type_or_any, false))
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
	type_name.symbol_name().filter(|word| {
		(is_kinded_type_or_any(word) || is_union(word)) && !crate::operators::FUNCTION_KEYWORDS.contains(word)
	})?;
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
			Node::List(parameters, _, _) => {
				Node::List([vec![items[0].clone()], parameters.clone()].concat(), bracket.clone(), separator.clone())
			}
			_ => head.clone(),
		},
		_ => head.clone(),
	}
}

/// The value converted to the declared result type; an arithmetic value is grouped, `(x*2) as int`, as written it
/// would be the ambiguous `x*2 as int`. The branches of a choice convert each, so an `error …` branch stays the
/// error (card braced-body: `def f(x) -> text { error "e" }` fails with e; `as text` would give the text "(error 'e')")
fn as_type(value: Node, type_name: &Node) -> Node {
	match value.drop_meta() {
		_ if is_error_value(&value) => value,
		Node::Key(condition, Op::Question, branches)
			if let Node::Key(chosen, Op::Colon, otherwise) = branches.drop_meta() =>
		{
			key(
				condition.as_ref().clone(),
				Op::Question,
				key(
					as_type(chosen.as_ref().clone(), type_name),
					Op::Colon,
					as_type(otherwise.as_ref().clone(), type_name),
				),
			)
		}
		Node::Key(then, Op::Else, otherwise) if let Node::Key(head, Op::Then, chosen) = then.drop_meta() => {
			let then = key(head.as_ref().clone(), Op::Then, as_type(chosen.as_ref().clone(), type_name));
			key(then, Op::Else, as_type(otherwise.as_ref().clone(), type_name))
		}
		_ if is_statement_block(&value) => converted_last(value, type_name),
		_ if type_name.symbol_name().is_some_and(is_checked_result) => checked_result(value, type_name),
		Node::Key(_, op, _) if op.is_arithmetic() => key(parens(vec![value]), Op::As, type_name.clone()),
		_ => key(value, Op::As, type_name.clone()),
	}
}

/// `{a; b}`: a block of statements, its last one the value; `{cube 3}` is the one statement `cube 3`
fn is_statement_block(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::Curly, separator) if !items.is_empty() && (*separator != Separator::Space || items.len() == 1))
}

/// `int or text`, `Circle or Square`: the name of an inline union (analyzer lower_inline_unions)
fn is_union(word: &str) -> bool {
	crate::analyzer::union_parts(word.trim_end_matches('?')).is_some()
}

/// A result checked rather than converted, where converting could lose something: a union (user 2026-10-12, `-> int or
/// text`) and a whole-number type (decision float-result: `-> int { 2.5 }` is no 2); `-> text { 3 }` still converts
pub(crate) fn is_checked_result(word: &str) -> bool {
	is_union(word) || (crate::analyzer::is_whole_type(word) && !crate::analyzer::is_bool_type(word))
}

/// `{the result: int or text = value; the result}`: the value admitted as a variable declared of the type is, a
/// literal when compiled, a fraction known at run time with "an int must be a whole number"
fn checked_result(value: Node, type_name: &Node) -> Node {
	let declared = key(key(symbol(crate::analyzer::CHECKED_RESULT), Op::Colon, type_name.clone()), Op::Assign, value);
	Node::List(vec![declared, symbol(crate::analyzer::CHECKED_RESULT)], Bracket::Curly, Separator::Semicolon)
}

/// `error "e"`
fn is_error_value(value: &Node) -> bool {
	match value.drop_meta() {
		Node::Error(_) => true,
		Node::List(items, _, _) => items.len() == 2 && items[0].symbol_name() == Some(ERROR_WORD),
		_ => false,
	}
}

fn is_return_statement(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, _, _) if is_return(items))
}

/// The body with every value it gives back converted: each `return e` and the last statement of a block
fn converted(body: &Node, type_name: &Node) -> Node {
	converted_last(converted_returns(body.drop_meta().clone(), type_name), type_name)
}

/// The last statement of a block converted, the value itself outside a block; a `return` is converted already
fn converted_last(body: Node, type_name: &Node) -> Node {
	match body.drop_meta().clone() {
		Node::List(mut items, Bracket::Curly, separator) if is_statement_block(&body) => {
			let last = items.pop().expect("not empty");
			items.push(if is_return_statement(&last) { last } else { as_type(last, type_name) });
			Node::List(items, Bracket::Curly, separator)
		}
		body if is_return_statement(&body) => body,
		// `{cube 3}`: its one statement, so the check reads `cube 3` as a call, not a block value
		Node::List(items, Bracket::Curly, Separator::Space) => {
			as_type(Node::List(items, Bracket::None, Separator::Space), type_name)
		}
		body => as_type(body, type_name),
	}
}

fn converted_returns(node: Node, type_name: &Node) -> Node {
	match node {
		Node::List(mut items, bracket, separator) if is_return(&items) => {
			let value = items.pop().expect("two items");
			items.push(as_type(value, type_name));
			Node::List(items, bracket, separator)
		}
		other => other.map_children(|child| converted_returns(child, type_name)),
	}
}

/// `square(x) as int := …` / `square(x) : int = …`: the head with its return type, as `typed_return`
fn return_typed_definition(node: &Node) -> Option<Node> {
	let Node::Key(target, op @ (Op::Assign | Op::Define), body) = node.drop_meta() else {
		return None;
	};
	let (head, type_name) = match target.drop_meta() {
		Node::Key(head, Op::As | Op::Colon, type_name) => (head.as_ref().clone(), type_name.as_ref().clone()),
		// `square(x) as int or text := …`
		target => {
			let (head, union) = crate::declarations::as_union_result(target)?;
			(head, symbol(&crate::analyzer::union_of_types(&union, &|_| true)?))
		}
	};
	// `f(w) as any := …`: a value of any kind converts to nothing (card def); `y: any = yes` stays a declaration
	if type_name.is_symbol(ANY_TYPE) && is_call_head(&head) {
		return Some(Node::Key(Box::new(head), *op, body.clone()));
	}
	typed_return(&type_name, &Node::Key(Box::new(head), *op, body.clone()))
}

/// `int age = 30` (C style) is `age: int = 30` (card bind-ignores), so every pass reads one form of a typed
/// declaration: a handler's `age += 1` the main-level variable, `input{ bind: age }` a number field
pub fn lower_prefixed_declarations(node: Node) -> Node {
	match node {
		Node::List(items, Bracket::None, _) if prefixed_declaration(&items).is_some() => {
			prefixed_declaration(&items).expect("guarded")
		}
		other => other.map_children(lower_prefixed_declarations),
	}
}

fn prefixed_declaration(items: &[Node]) -> Option<Node> {
	let [type_name, declaration] = items else { return None };
	type_name.symbol_name().filter(|word| is_kinded_type_or_any(word))?;
	let Node::Key(name, Op::Assign, value) = declaration.drop_meta() else {
		return None;
	};
	name.drop_meta().symbol_name()?;
	Some(Node::Key(
		Box::new(key(name.drop_meta().clone(), Op::Colon, type_name.drop_meta().clone())),
		Op::Assign,
		value.clone(),
	))
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
		meta @ Node::Meta { .. } => meta.under_meta(lower),
		other => other,
	}
}
