//! The loop headers the parser keeps as written, given their meaning first, before any other pass reads the loop:
//! `for friend in xs` and `for int in xs` visit only the items of that type, `for (it>2) in xs` and
//! `for (even number) in xs` only the items the condition holds for (each filter announced, P46); `for chars in text`
//! walks the text by that unit, `for word in text`, `for line in text` and `for byte in text` by words, lines and bytes.

use crate::errors::diagnostic::{ask, reading, Ask, Fallback};
use crate::lowering::nodes::arguments_rewritten;
use crate::node::{call, parens, Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::{for_in_loop, mentions};
use crate::words::SPLIT_WORD;
use crate::words::{FOR_WORD, IMPLICIT_PARAMETER, INSTANCE_OF, IN_WORD, IS_TYPE};
use std::collections::HashSet;

/// The got-it topic of a filtering loop (`for friend in xs`, `for (it>2) in xs`)
pub(crate) const FILTER_LOOP_TOPIC: &str = "for-filter";
/// The unit words a for loop walks a text by, the item being `it` (wiki/string.md)
const UNIT_LOOP_WORDS: [&str; 4] = ["chars", "characters", "codepoints", "bytes"];
const BYTES_WORD: &str = "bytes";
const BYTE_WORD: &str = "byte";
const WORD_UNIT: &str = "word";
/// The loop variable names that walk a text by a unit other than the character, and the text between two units
const TEXT_UNIT_NAMES: [(&str, &str); 2] = [(WORD_UNIT, " "), ("line", "\n")];
const FILTER_WORD: &str = "filter";
/// Built-in adjectives of a loop filter `(even number)`, when no function of that name is defined
const EVEN_WORD: &str = "even";
const ODD_WORD: &str = "odd";

/// The type test of a loop `for T in xs` carries its loop: the iterable and the got-it question announcing the filter
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FilterLoop {
	pub question: Ask,
	pub iterable: Node,
}

/// Every loop header of the program given its meaning, inner loops first
pub fn lower(program: Node) -> Node {
	let mut headers = Headers {
		declared_types: crate::analyzer::declared_classes(&program),
		functions: crate::analyzer::function_definitions(&program).user_functions.into_keys().collect(),
		text_variables: HashSet::new(),
	};
	headers.lowered(program)
}

/// What a header's meaning depends on. The text variables are those given a text before the loop ends, in source order,
/// as the parser knew them when it read the loop
struct Headers {
	declared_types: HashSet<String>,
	functions: HashSet<String>,
	text_variables: HashSet<String>,
}

impl Headers {
	fn lowered(&mut self, node: Node) -> Node {
		let node = node.map_children(|child| self.lowered(child));
		self.note_variable(&node);
		let Node::List(items, Bracket::None, Separator::Space) = &node else {
			return node;
		};
		match items.as_slice() {
			[for_word, variable, in_word, iterable, body]
				if for_word.is_symbol(FOR_WORD) && in_word.is_symbol(IN_WORD) =>
			{
				let at = crate::errors::diagnostic::position(&node).unwrap_or_default();
				let (variable, iterable, body) = (variable.clone(), iterable.clone(), body.clone());
				self.loop_meaning(variable, iterable, body, at).unwrap_or_else(|error| error)
			}
			_ => node,
		}
	}

	/// `l = "en"` or `k:text` (a typed parameter or declaration) makes l, k text variables
	fn note_variable(&mut self, node: &Node) {
		let Node::Key(name, op, value) = node.drop_meta() else {
			return;
		};
		let Node::Symbol(name) = name.drop_meta() else { return };
		let holds_text = match (op, value.drop_meta()) {
			(Op::Assign | Op::Define, Node::Text(_)) => true,
			(Op::Colon, Node::Symbol(type_word)) => crate::type_tests::is_text_type_word(type_word),
			_ => false,
		};
		if holds_text {
			self.text_variables.insert(name.clone());
		}
	}

	/// The loop `for variable in iterable body` as it runs; Err the error of a filter the program refuses
	fn loop_meaning(&self, variable: Node, iterable: Node, body: Node, at: (usize, usize)) -> Result<Node, Node> {
		let it = Node::Symbol(IMPLICIT_PARAMETER.to_string());
		// `for (it>2) in xs`, `for (even number) in xs`: the condition the parser kept in round brackets
		if let Some(condition) = self.loop_condition(&variable) {
			let shown = crate::normalize::operand_text(&condition);
			let collection = crate::normalize::operand_text(&iterable);
			let header = (format!("for ({shown}) in {collection}"), filter_loop_header(&collection, &shown));
			let body = filtered_body(
				&format!("for ({shown}) in …"),
				&filter_loop_block(&shown),
				Some(header),
				condition,
				body,
				at,
			)?;
			return Ok(for_in_loop(it, iterable, body));
		}
		let (variable, body) = match variable.drop_meta() {
			// `for each byte in "Hello"` (typed iteration, wiki/Purpose.md): over a text the type word byte names the unit,
			// the UTF-8 bytes, rather than filtering the characters (none is a byte)
			Node::Symbol(name) if name == BYTE_WORD && self.is_text(&iterable) => {
				return Ok(for_in_loop(variable, bytes_of(&iterable), body));
			}
			// `for friend in xs`: a declared type's name visits only its instances, as `it`; the name in the body is the
			// item (P46), a call `friend(…)` still constructs
			Node::Symbol(name) if self.declared_types.contains(name) => {
				let test = call(INSTANCE_OF, vec![it.clone(), Node::Text(name.clone())]);
				let header = type_filter_header(name, &iterable, &body);
				let body = item_named(body, name, &it);
				let written = format!("for {name} in …");
				(it, filtered_body(&written, &filter_loop_block(&format!("it is {name}")), header, test, body, at)?)
			}
			// `for number in xs`: a type word visits only the items of that type, under its own name; a literal list whose
			// items all are of the type needs no filter
			Node::Symbol(name)
				if crate::analyzer::type_word_kind(name).is_some() && !literal_items_of_type(&iterable, name) =>
			{
				let test = self.type_test(&variable, name).expect("a type word");
				let header = type_filter_header(name, &iterable, &body);
				let body = type_filtered_body(name, &iterable, header, test, body, at);
				(variable, body)
			}
			_ => (variable, body),
		};
		// `for chars in text: print it`: a unit word walks the text by that unit, the item is `it`
		if let Some((unit, iterable)) = unit_iteration(&variable, &iterable, &body) {
			return Ok(for_in_loop(unit, iterable, body));
		}
		let iterable = self.walked_text_units(&variable, iterable);
		Ok(for_in_loop(variable, iterable, body))
	}

	/// The condition of `for (it>2) in xs`, `for (even number) in xs`; None for any other loop variable
	fn loop_condition(&self, variable: &Node) -> Option<Node> {
		let Node::List(items, Bracket::Round, Separator::None) = variable.drop_meta() else {
			return None;
		};
		let [condition] = items.as_slice() else {
			return None;
		};
		match mentions(condition, IMPLICIT_PARAMETER) {
			true => Some(condition.clone()),
			false => self.adjective_condition(condition),
		}
	}

	/// `(even number)` in a loop header: `even(it)` (built in: `it%2==0`, `odd` its opposite, else the user's function) and
	/// `it is number`; None for anything else
	fn adjective_condition(&self, words: &Node) -> Option<Node> {
		// the two words, spaced or read as the call `even(number)`
		let Node::List(items, Bracket::None | Bracket::Round, Separator::Space | Separator::None) = words.drop_meta()
		else {
			return None;
		};
		let [adjective, type_word] = items.as_slice() else {
			return None;
		};
		let (Node::Symbol(adjective), Node::Symbol(type_name)) = (adjective.drop_meta(), type_word.drop_meta()) else {
			return None;
		};
		let it = Node::Symbol(IMPLICIT_PARAMETER.to_string());
		let type_test = self.type_test(&it, type_name)?;
		let parity = |op: Op| {
			Node::Key(
				Box::new(Node::Key(Box::new(it.clone()), Op::Mod, Box::new(Node::int(2)))),
				op,
				Box::new(Node::int(0)),
			)
		};
		let adjective_test = match adjective.as_str() {
			word if self.functions.contains(word) => call(word, vec![it.clone()]),
			EVEN_WORD => parity(Op::Eq),
			ODD_WORD => parity(Op::Ne),
			word => call(word, vec![it.clone()]),
		};
		Some(Node::Key(Box::new(type_test), Op::And, Box::new(adjective_test)))
	}

	/// `value is T` for a loop filter: a declared type by its instances, a type word by its runtime kind; None for any other word
	fn type_test(&self, value: &Node, type_name: &str) -> Option<Node> {
		let test = if self.declared_types.contains(type_name) {
			INSTANCE_OF
		} else if crate::analyzer::type_word_kind(type_name).is_some() {
			IS_TYPE
		} else {
			return None;
		};
		Some(call(test, vec![value.clone(), Node::Text(type_name.to_string())]))
	}

	/// `for word in text`, `for line in text` (wiki/plural.md): over a text the loop variable's name picks the unit, words
	/// between runs of spaces or lines (as lib/text.warp's words and lines); any other name walks the characters
	fn walked_text_units(&self, variable: &Node, iterable: Node) -> Node {
		let is_text = self.is_text(&iterable);
		let unit =
			variable.drop_meta().symbol_name().and_then(|name| TEXT_UNIT_NAMES.iter().find(|(unit, _)| *unit == name));
		let (Some((unit, separator)), true) = (unit, is_text) else {
			return iterable;
		};
		let parts = call(SPLIT_WORD, vec![iterable, Node::Text(separator.to_string())]);
		if *unit != WORD_UNIT {
			return parts;
		}
		let part = Node::Symbol(format!("{unit}·part"));
		let non_empty = Node::Key(
			Box::new(part.clone()),
			Op::Arrow,
			Box::new(Node::Key(Box::new(part), Op::Ne, Box::new(Node::Text(String::new())))),
		);
		Node::Key(Box::new(parts), Op::Dot, Box::new(call(FILTER_WORD, vec![non_empty])))
	}

	/// A text literal or a variable given a text
	fn is_text(&self, iterable: &Node) -> bool {
		match iterable.drop_meta() {
			Node::Text(_) => true,
			Node::Symbol(name) => self.text_variables.contains(name),
			_ => false,
		}
	}
}

/// The body of a filtering loop: run only when `test` holds; the filter is announced once (got-it warning, P46).
/// `header` is the loop's header as written and its explicit form, the fix ("I meant: …"); None when the body
/// needs the edit too
fn filtered_body(
	written: &str,
	explicit: &str,
	header: Option<(String, String)>,
	test: Node,
	body: Node,
	at: (usize, usize),
) -> Result<Node, Node> {
	ask(&filter_question(written, explicit, header, at))?;
	Ok(guarded_body(test, body))
}

/// The got-it question announcing a filtering loop
fn filter_question(written: &str, explicit: &str, header: Option<(String, String)>, at: (usize, usize)) -> Ask {
	let question = format!("`{written}` visits only the items that pass its filter");
	let filter = reading("filter the items", explicit);
	let readings = vec![match header {
		Some((written, replacement)) => filter.replacing(written, replacement),
		None => filter,
	}];
	Ask::new(FILTER_LOOP_TOPIC, question, readings, Fallback::Warning).written(written).at(at.0, at.1)
}

/// `for T in xs` of a type word T, which visits only the items of type T: the type test carries the loop, so the emitter
/// decides it from the items' static type and announces the filter only when it is left to run time (`for char in s`
/// of a text s filters nothing)
fn type_filtered_body(
	name: &str,
	iterable: &Node,
	header: Option<(String, String)>,
	test: Node,
	body: Node,
	at: (usize, usize),
) -> Node {
	let question =
		filter_question(&format!("for {name} in …"), &filter_loop_block(&format!("it is {name}")), header, at);
	guarded_body(test.with_meta_data(FilterLoop { question, iterable: iterable.clone() }), body)
}

/// `{ if test then body }`, the body of a filtering loop
fn guarded_body(test: Node, body: Node) -> Node {
	let guarded =
		Node::Key(Box::new(Node::Key(Box::new(Node::Empty), Op::If, Box::new(test))), Op::Then, Box::new(body));
	Node::List(vec![guarded], Bracket::Curly, Separator::Semicolon)
}

/// `for int in xs` as the explicit filter `for xs.filter(it is int)`, the body unchanged; None when the body
/// names the item by the type word (then the body would change too)
fn type_filter_header(name: &str, iterable: &Node, body: &Node) -> Option<(String, String)> {
	if mentions(body, name) {
		return None;
	}
	let collection = crate::normalize::operand_text(iterable);
	Some((format!("for {name} in {collection}"), filter_loop_header(&collection, &format!("it is {name}"))))
}

/// The explicit header of a filtering loop, `for xs.filter(it > 2)`: no loop variable, so the body keeps reading the
/// item as `it`, which a loop may not bind by name (implicit-words-reserved)
fn filter_loop_header(collection: &str, test: &str) -> String {
	format!("for {collection}.filter({test})")
}

/// The explicit block form of a filtering loop, `for … { if it > 2 { … } }`
pub(crate) fn filter_loop_block(test: &str) -> String {
	format!("for … {{ if {test} {{ … }} }}")
}

/// `name` in a loop body as the item: every bare use, not the head of a call `name(…)`
fn item_named(node: Node, name: &str, item: &Node) -> Node {
	match node {
		Node::Symbol(symbol) if symbol == name => item.clone(),
		Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(head)) if head == name) => {
			arguments_rewritten(items, Bracket::Round, Separator::None, |part| item_named(part, name, item))
		}
		other => other.map_children(|child| item_named(child, name, item)),
	}
}

/// A list literal whose items all are literals of the type: `[1, 2]` for `number`
fn literal_items_of_type(iterable: &Node, type_name: &str) -> bool {
	let items = match iterable.drop_meta() {
		Node::List(items, Bracket::Square, _) => items,
		// `for char in "abc"`: every item of a text is a char
		Node::Text(_) => return crate::type_tests::canonical_spec_word(type_name) == crate::words::CODEPOINT,
		_ => return false,
	};
	let spec = crate::analyzer::type_word_kind(type_name).map(|_| type_name).unwrap_or(type_name);
	items.iter().all(|item| match item.drop_meta() {
		Node::Number(_) | Node::Text(_) | Node::Char(_) => {
			crate::type_tests::type_matches(&item.drop_meta().kind().to_string(), spec)
		}
		_ => false,
	})
}

/// `for chars in text {…it…}` (also characters, codepoints, bytes): the variable `it` over `chars(text)`, or over the
/// bytes `(0..text.bytes).map(i => byte_at(text, i))`; None when the body reads the word itself (a variable named chars)
fn unit_iteration(variable: &Node, iterable: &Node, body: &Node) -> Option<(Node, Node)> {
	let Node::Symbol(word) = variable.drop_meta() else {
		return None;
	};
	if !UNIT_LOOP_WORDS.contains(&word.as_str()) || body.mentions(word) {
		return None;
	}
	let walked = if word == BYTES_WORD { bytes_of(iterable) } else { call("chars", vec![iterable.clone()]) };
	Some((Node::Symbol(IMPLICIT_PARAMETER.to_string()), walked))
}

/// The UTF-8 bytes of a text, `(0..text.bytes).map(i => byte_at(text, i))`
fn bytes_of(text: &Node) -> Node {
	let offset = Node::Symbol("byte·offset".to_string());
	let count = Node::Key(Box::new(text.clone()), Op::Dot, Box::new(Node::Symbol(BYTES_WORD.to_string())));
	let offsets = Node::Key(Box::new(Node::int(0)), Op::Range, Box::new(count));
	let byte = Node::Key(Box::new(offset.clone()), Op::Arrow, Box::new(call("byte_at", vec![text.clone(), offset])));
	Node::Key(Box::new(parens(vec![offsets])), Op::Dot, Box::new(call("map", vec![byte])))
}
