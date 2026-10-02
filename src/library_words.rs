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
use crate::wasp_parser::{parse, ASSERT_MARKER, TRY_MARKER};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};

/// The map words, named after their runtime functions: `m.keys()`, `m.values()`, `x in xs` (`xs.has(x)`, Python's and
/// JavaScript's spellings too) and `m.get(k)`/`m.get(k, default)`. A key is found by its letters, quoted or not.
pub const MAP_KEYS: &str = "map_keys";
pub const MAP_VALUES: &str = "map_values";
/// map_entries(xs): the `key:value` entries of a map as a list (a one-entry map `{a:1}` is the entry itself), else xs
pub const MAP_ENTRIES: &str = "map_entries";
pub const COLLECTION_CONTAINS: &str = "collection_contains";
pub const MAP_GET_OR: &str = "map_get_or";
pub const MAP_WORD_FUNCTIONS: [&str; 5] = [MAP_KEYS, MAP_VALUES, MAP_ENTRIES, COLLECTION_CONTAINS, MAP_GET_OR];
const IN_WORD: &str = "in";
const FOR_WORD: &str = "for";

/// Canonical word and the spellings that mean it
const SYNONYMS: [(&str, &[&str]); 16] = [
	(MAP_KEYS, &["keys"]),
	(MAP_VALUES, &["values"]),
	(MAP_ENTRIES, &[]),
	(COLLECTION_CONTAINS, &["contains", "has", "includes"]),
	(MAP_GET_OR, &["get"]),
	("chars", &[]),
	("upper", &["uppercase"]),
	("lower", &["lowercase"]),
	("reverse", &[]),
	("sort", &[]),
	("split", &[]),
	("join", &[]),
	("first", &[]),
	("last", &[]),
	(SLICE, &[]),
	(COPY, &["clone"]),
];
const SUM: &str = "sum";

/// Words the emitter implements as runtime functions, with the number of arguments including the receiver
pub const RUNTIME_WORDS: [(&str, usize); 14] = [
	("upper", 1), ("lower", 1), ("reverse", 1), ("sort", 1), ("split", 2), ("join", 2), ("chars", 1), (FIELD_WITH, 3),
	(MAP_KEYS, 1), (MAP_VALUES, 1), (MAP_ENTRIES, 1), (COLLECTION_CONTAINS, 2), (MAP_GET_OR, 3), (SLICE, 3),
];
/// `slice(x, start, end)`: the items or characters start…end-1, 0-based (`a[1:3]`, `s.slice(1)`)
pub const SLICE: &str = "slice";
/// Trailing arguments a word may leave out, passed as ø: `m.get(k)` is ø for a missing key, `s.slice(2)` slices to the end
const OPTIONAL_ARGUMENTS: [(&str, usize); 2] = [(MAP_GET_OR, 1), (SLICE, 1)];
/// `b = a.copy()`: values are never shared, so the copy is the value itself
const COPY: &str = "copy";
/// `field_with(object, "name", value)`: a copy of the object with the field set; what `object.name = value` lowers to
pub const FIELD_WITH: &str = "field_with";

/// Source of the words expanded here; `word_argument` is the receiver, `word_tmp` a temporary that holds it once
const EXPANDED_WORDS: [(&str, &str); 3] = [
	("first", "word_tmp#1"),
	("last", "word_tmp#(count(word_tmp))"),
	(SUM, "(word_sum=0; for word_item in word_tmp {word_sum = word_sum + word_item}; word_sum)"),
];
/// Hidden variables and placeholders of the `try`/`assert` templates
const TRY_TEMPORARY: &str = "try_tmp";
const TRY_VALUE_PLACEHOLDER: &str = "try_placeholder_value";
const TRY_FALLBACK_PLACEHOLDER: &str = "try_placeholder_fallback";
const LIST_PLACEHOLDER: &str = "try_placeholder_list";
const INDEX_PLACEHOLDER: &str = "try_placeholder_index";
const DIVIDEND_PLACEHOLDER: &str = "try_placeholder_dividend";
const DIVISOR_PLACEHOLDER: &str = "try_placeholder_divisor";
const ASSERT_CONDITION_PLACEHOLDER: &str = "assert_placeholder_condition";
const RECEIVER_PLACEHOLDER: &str = "word_argument";
const LOOKUP_PLACEHOLDER: &str = "word_lookup";
const TEMPORARY: &str = "word_tmp";

/// Library words whose result is always a text, and those whose result is always a list
const TEXT_RESULT_WORDS: [&str; 3] = ["upper", "lower", "join"];
const LIST_RESULT_WORDS: [&str; 4] = ["chars", MAP_KEYS, MAP_VALUES, MAP_ENTRIES];

pub fn result_kind(word: &str) -> Option<crate::type_kinds::Kind> {
	use crate::type_kinds::Kind;
	if TEXT_RESULT_WORDS.contains(&word) {
		Some(Kind::Text)
	} else {
		LIST_RESULT_WORDS.contains(&word).then_some(Kind::List)
	}
}

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
	let mut assigned_objects = HashMap::new();
	collect_assigned_objects(&node, &mut assigned_objects);
	let objects = assigned_objects.into_iter().filter_map(|(name, literal)| Some((name, literal?))).collect();
	Lowering { context, shadowed, objects, temporaries: Cell::new(0) }.expand(node)
}

/// `object.name`, `name of object`, `object["name"]`: the entry named `name`, a subscript by a text key
fn field_lookup(object: &Node, name: &str, position: &Node) -> Node {
	let key = match position {
		Node::Meta { data, .. } => Node::Meta { node: Box::new(Node::Text(name.to_string())), data: data.clone() },
		_ => Node::Text(name.to_string()),
	};
	crate::wasp_parser::subscript(object.clone(), key)
}

/// A field lookup by a name, as `field_lookup` builds it: its value is an object whenever the field holds one
fn is_field_lookup(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Hash, index) if crate::wasp_parser::subscript_key(index).and_then(field_name).is_some())
}

/// The entries of an object literal `{a:1 b:2}` (or `{a:1}`): every item is a `key:value`
fn object_entries(node: &Node) -> Option<Vec<(String, Node)>> {
	let single;
	let items: &[Node] = match node.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items, // `{}` is the empty object, grown by `d["k"] = v`
		Node::Key(_, Op::Colon, _) => {
			single = [node.clone()];
			&single
		}
		_ => return None,
	};
	items
		.iter()
		.map(|item| match item.drop_meta() {
			Node::Key(key, Op::Colon, value) => Some((field_name(key)?, value.as_ref().clone())),
			_ => None,
		})
		.collect()
}

fn field_name(key: &Node) -> Option<String> {
	match key.drop_meta() {
		Node::Symbol(name) | Node::Text(name) => Some(name.clone()),
		Node::Char(letter) => Some(letter.to_string()), // `{"A": 1}`: a quoted one-letter key
		_ => None,
	}
}

/// Variables that are only ever assigned an object literal, with that literal; `None` for a variable assigned anything else too
fn collect_assigned_objects(node: &Node, objects: &mut HashMap<String, Option<Node>>) {
	match node.drop_meta() {
		Node::Key(target, Op::Assign | Op::Define, value) => {
			if let Node::Symbol(name) = target.drop_meta() {
				let copied = match value.drop_meta() {
					Node::Symbol(other) => objects.get(other).cloned().flatten(), // `q=p` is the same object
					_ => None,
				};
				let literal = copied.or_else(|| object_entries(value).map(|_| value.as_ref().clone()));
				let both_objects = literal.is_some() && objects.get(name).is_none_or(|earlier| earlier.is_some());
				objects.insert(name.clone(), if both_objects { literal } else { None });
			}
			collect_assigned_objects(value, objects);
		}
		Node::Key(left, _, right) => {
			collect_assigned_objects(left, objects);
			collect_assigned_objects(right, objects);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_assigned_objects(item, objects)),
		_ => {}
	}
}

pub(crate) fn collect_assigned_names(node: &Node, names: &mut HashSet<String>) {
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
	objects: HashMap<String, Node>,
	temporaries: Cell<usize>,
}

impl Lowering {
	fn expand(&self, node: Node) -> Node {
		match node {
			Node::List(items, Bracket::Round, _) if items.len() == 3 && is_marker(&items[0], TRY_MARKER) => {
				self.lower_try(self.expand(items[1].clone()), self.expand(items[2].clone()))
			}
			Node::List(items, Bracket::Round, _) if items.len() == 3 && is_marker(&items[0], ASSERT_MARKER) => {
				self.lower_assert(self.expand(items[1].clone()), self.expand(items[2].clone()))
			}
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				if let Some(walk) = self.for_over_map(&items) {
					return walk;
				}
				self.word_call(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Key(left, Op::Dot, right) => {
				let (left, right) = (self.expand(*left), self.expand_method(*right));
				self.method_call(&left, &right).unwrap_or(Node::Key(Box::new(left), Op::Dot, Box::new(right)))
			}
			Node::Key(left, Op::Assign, right) => {
				let (left, right) = (self.expand(*left), self.expand(*right));
				self.field_assignment(&left, &right).unwrap_or(Node::Key(Box::new(left), Op::Assign, Box::new(right)))
			}
			Node::Key(left, Op::SafeDot, right) => {
				let (receiver, word) = (self.expand(*left), *right);
				self.safe_lookup(receiver, word)
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
		if let Some(lookup) = self.of_lookup(items) {
			return Some(lookup);
		}
		if let Some(membership) = self.membership(items) {
			return Some(membership);
		}
		let head = match items.first()?.drop_meta() {
			Node::Symbol(name) => name,
			_ => return None,
		};
		let word = self.library_word(head)?;
		if let [_, of, rest @ ..] = items {
			if matches!(of.drop_meta(), Node::Symbol(word) if word == "of") && !rest.is_empty() {
				// `first of xs` is `first(xs)`
				let argument = match rest {
					[single] => single.clone(),
					_ => Node::List(rest.to_vec(), Bracket::None, Separator::Space),
				};
				return Some(self.call(word, &items[0], vec![argument], false));
			}
		}
		let is_call = call_name(items, bracket, separator).is_some();
		let is_prefix = *bracket == Bracket::None && *separator == Separator::Space && items.len() > 1;
		if !is_call && !is_prefix {
			return None;
		}
		Some(self.call(word, &items[0], items[1..].to_vec(), is_prefix && !is_call))
	}

	/// `for k in m` walks the keys of a map, as in Python; `for k, v in m` and `for (k, v) in m` bind each value too:
	/// `for k in map_keys(m) {v = m[k]; …}`
	fn for_over_map(&self, items: &[Node]) -> Option<Node> {
		let [keyword, names, in_word, rest @ ..] = items else { return None };
		if !is_marker(keyword, FOR_WORD) || !is_marker(in_word, IN_WORD) {
			return None;
		}
		let (map, body) = match rest {
			[map, body] => (map.clone(), body.clone()),
			[colon] => match colon.drop_meta() {
				Node::Key(map, Op::Colon, body) => (map.as_ref().clone(), body.as_ref().clone()),
				_ => return None,
			},
			_ => return None,
		};
		self.object_literal(&map)?;
		let (key, value) = match names.drop_meta() {
			Node::Symbol(_) => (names.clone(), None),
			Node::List(parts, Bracket::Round | Bracket::None, _) => match parts.as_slice() {
				[key, value] if [key, value].iter().all(|name| matches!(name.drop_meta(), Node::Symbol(_))) => (key.clone(), Some(value.clone())),
				_ => return None,
			},
			_ => return None,
		};
		let body = match value {
			None => body,
			Some(value) => {
				let lookup = Node::Key(Box::new(value), Op::Assign, Box::new(crate::wasp_parser::subscript(map.clone(), key.clone())));
				Node::List([vec![lookup], crate::for_loop::block_items(&body)].concat(), Bracket::Curly, Separator::Semicolon)
			}
		};
		let keys = self.call(MAP_KEYS, keyword, vec![map], false);
		Some(Node::List(vec![keyword.clone(), key, in_word.clone(), keys, body], Bracket::None, Separator::Space))
	}

	/// `x in xs`: whether a map has the key x or a list the element x; `byte in t` and `"äb" in bytes` count text units
	fn membership(&self, items: &[Node]) -> Option<Node> {
		let [element, in_word, collection] = items else { return None };
		let names_unit = |side: &Node| match side.drop_meta() {
			Node::Key(_, Op::Hash, unit) => crate::analyzer::text_unit(&unit.name()).is_some(),
			other => crate::analyzer::text_unit(&other.name()).is_some(),
		};
		if !is_marker(in_word, IN_WORD) || names_unit(element) || names_unit(collection) {
			return None;
		}
		Some(self.call(COLLECTION_CONTAINS, in_word, vec![collection.clone(), element.clone()], false))
	}

	/// `x.word` and `x.word(args)`; an unknown word on a value is an error
	fn method_call(&self, receiver: &Node, method: &Node) -> Option<Node> {
		let (word_node, arguments) = match method.drop_meta() {
			Node::Symbol(_) => (method, vec![]),
			Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => (&items[0], items[1..].to_vec()),
			_ => return None,
		};
		let Node::Symbol(name) = word_node.drop_meta() else { return None };
		let has_arguments = !arguments.is_empty();
		let literal = self.object_literal(receiver);
		let is_field = literal.as_ref().and_then(object_entries).is_some_and(|entries| entries.iter().any(|(field, _)| field == name));
		if is_field && !has_arguments {
			return Some(field_lookup(receiver, name, word_node));
		}
		// `x.chars` stays the count of characters; only `chars x` and `chars(x)` are the list
		if let Some(word) = self.library_word(name).filter(|_| counting_method(name, &self.context).is_none()) {
			return Some(self.call(word, word_node, [vec![receiver.clone()], arguments].concat(), false));
		}
		let is_known = counting_method(name, &self.context).is_some() || is_append_method(name) || self.context.user_functions.contains_key(name);
		let is_object = literal.is_some() || is_field_lookup(receiver);
		if is_object && !is_known && !has_arguments {
			return Some(field_lookup(receiver, name, word_node));
		}
		let is_value = matches!(receiver.drop_meta(), Node::Symbol(_) | Node::Text(_) | Node::Char(_) | Node::Number(_) | Node::List(_, Bracket::Square, _));
		(!is_known && is_value).then(|| Diagnostic::at(word_node, format!("undefined function: {name}")).into_error())
	}

	/// `object.name = value` is `object = field_with(object, "name", value)`; a nested path updates the objects on the way
	/// (`p.b.c = 4` is `p = field_with(p, "b", field_with(p.b, "c", 4))`)
	fn field_assignment(&self, target: &Node, value: &Node) -> Option<Node> {
		let Node::Key(base, Op::Hash, index) = target.drop_meta() else { return None };
		let name = match crate::wasp_parser::subscript_key(index)?.drop_meta() {
			Node::Text(name) => name.clone(),
			Node::Char(letter) => letter.to_string(),
			_ => return None,
		};
		let updated = Node::List(vec![Node::Symbol(FIELD_WITH.to_string()), base.as_ref().clone(), Node::Text(name), value.clone()], Bracket::Round, Separator::None);
		match base.drop_meta() {
			Node::Symbol(_) => Some(Node::Key(base.clone(), Op::Assign, Box::new(updated))),
			_ if is_field_lookup(base) => self.field_assignment(base, &updated),
			_ => None,
		}
	}

	/// `receiver?.name`: ø when the receiver is ø, else the field
	fn safe_lookup(&self, receiver: Node, word: Node) -> Node {
		let Node::Symbol(name) = word.drop_meta() else {
			return Diagnostic::at(&word, "?. needs a field name after it").into_error();
		};
		let number = self.temporaries.get();
		self.temporaries.set(number + 1);
		let temporary = format!("safe_tmp_{number}");
		let program = parse(&format!("({temporary}={RECEIVER_PLACEHOLDER}; if {temporary} == ø then ø else {LOOKUP_PLACEHOLDER})"));
		let lookup = field_lookup(&Node::Symbol(temporary), name, &word);
		substitute(substitute(program, RECEIVER_PLACEHOLDER, &receiver), LOOKUP_PLACEHOLDER, &lookup)
	}

	/// The object literal a node stands for: the literal itself, a variable only ever assigned one, or a field of such an object
	fn object_literal(&self, node: &Node) -> Option<Node> {
		match node.drop_meta() {
			Node::Symbol(name) => self.objects.get(name).cloned(),
			Node::Key(base, Op::Hash, index) => {
				let field = field_name(crate::wasp_parser::subscript_key(index)?)?;
				let (_, value) = object_entries(&self.object_literal(base)?)?.into_iter().find(|(name, _)| *name == field)?;
				object_entries(&value).map(|_| value)
			}
			other => object_entries(other).map(|_| other.clone()),
		}
	}

	/// `word of object`, `c of b of p`: the lookup of a field, when the object is one; `None` for anything else
	fn of_lookup(&self, items: &[Node]) -> Option<Node> {
		let [word_node, of, rest @ ..] = items else { return None };
		let Node::Symbol(name) = word_node.drop_meta() else { return None };
		if !matches!(of.drop_meta(), Node::Symbol(word) if word == "of") || rest.is_empty() {
			return None;
		}
		let object = match rest {
			[single] => single.clone(),
			[_, next_of, ..] if matches!(next_of.drop_meta(), Node::Symbol(word) if word == "of") => self.of_lookup(rest)?,
			_ => return None,
		};
		let literal = self.object_literal(&object);
		let is_field = literal.as_ref().and_then(object_entries).is_some_and(|entries| entries.iter().any(|(field, _)| field == name));
		let is_object = literal.is_some() || is_field_lookup(&object);
		let is_other_word = self.library_word(name).is_some() || counting_method(name, &self.context).is_some();
		(is_object && (is_field || !is_other_word)).then(|| field_lookup(&object, name, word_node))
	}

	/// The call of `word` with the receiver and its arguments; arguments are the items after the word for the prefix form
	fn call(&self, word: &'static str, head: &Node, arguments: Vec<Node>, is_prefix: bool) -> Node {
		let mut arguments = if is_prefix { merge_prefix_arguments(word, arguments) } else { arguments };
		let wanted = arity(word);
		let optional = OPTIONAL_ARGUMENTS.iter().find(|(name, _)| *name == word).map_or(0, |(_, optional)| *optional);
		if arguments.len() < wanted && arguments.len() + optional >= wanted {
			arguments.resize(wanted, Node::Empty);
		}
		if let (COPY, [receiver]) = (word, arguments.as_slice()) {
			crate::normalize::hint(&format!("{}.{}()", receiver.serialize(), head.serialize()), &receiver.serialize(), "values are never shared: b = a already copies");
			return receiver.clone();
		}
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

	/// A source template with `hidden` variables made unique, its placeholders replaced by the given nodes
	fn from_template(&self, template: &str, replacements: &[(&str, &Node)]) -> Node {
		let number = self.temporaries.get();
		self.temporaries.set(number + 1);
		let mut program = parse(&template.replace(TRY_TEMPORARY, &format!("{TRY_TEMPORARY}_{number}")));
		for (placeholder, replacement) in replacements {
			program = substitute(program, placeholder, replacement);
		}
		program
	}

	/// `try X else Y`: X, or Y when X fails. Errors are values: an Error result is replaced. An index out of range and a
	/// division or modulo by zero directly under `try` are checked before they trap; a trap deeper inside X stays a trap.
	fn lower_try(&self, guarded: Node, fallback: Node) -> Node {
		let (value, fallback_placeholder) = (TRY_VALUE_PLACEHOLDER, TRY_FALLBACK_PLACEHOLDER);
		match guarded.drop_meta() {
			Node::Key(target, Op::Assign, assigned) if matches!(target.drop_meta(), Node::Symbol(_)) => {
				Node::Key(target.clone(), Op::Assign, Box::new(self.lower_try(assigned.as_ref().clone(), fallback)))
			}
			Node::Key(list, Op::Hash, index) if !matches!(list.drop_meta(), Node::Empty) => self.from_template(
				&format!("(try_tmp_list={LIST_PLACEHOLDER}; try_tmp_index={INDEX_PLACEHOLDER}; if try_tmp_index >= 1 and try_tmp_index <= count(try_tmp_list) {{try_tmp_list#try_tmp_index}} else {{{fallback_placeholder}}})"),
				&[(LIST_PLACEHOLDER, list), (INDEX_PLACEHOLDER, index), (fallback_placeholder, &fallback)],
			),
			Node::Key(dividend, op @ (Op::Div | Op::Mod | Op::Rem), divisor) => self.from_template(
				&format!("(try_tmp_divisor={DIVISOR_PLACEHOLDER}; if try_tmp_divisor==0 {{{fallback_placeholder}}} else {{{DIVIDEND_PLACEHOLDER} {op} try_tmp_divisor}})"),
				&[(DIVISOR_PLACEHOLDER, divisor), (DIVIDEND_PLACEHOLDER, dividend), (fallback_placeholder, &fallback)],
			),
			_ => self.from_template(
				&format!("(try_tmp_value={value}; if is_error(try_tmp_value) {{{fallback_placeholder}}} else {{try_tmp_value}})"),
				&[(value, &guarded), (fallback_placeholder, &fallback)],
			),
		}
	}

	/// `assert C else X` is 1 when C holds and the Error X otherwise; without a message the error says the assertion failed
	fn lower_assert(&self, condition: Node, message: Node) -> Node {
		let message = match message.drop_meta() {
			Node::Empty => Node::Text(format!("assertion failed: {}", condition.serialize())),
			_ => message,
		};
		let condition_placeholder = ASSERT_CONDITION_PLACEHOLDER;
		self.from_template(
			&format!("(if {condition_placeholder} {{1}} else {{error({TRY_FALLBACK_PLACEHOLDER})}})"),
			&[(condition_placeholder, &condition), (TRY_FALLBACK_PLACEHOLDER, &message)],
		)
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

pub(crate) fn substitute(node: Node, placeholder: &str, replacement: &Node) -> Node {
	match node {
		Node::Symbol(name) if name == placeholder => replacement.clone(),
		Node::Key(left, op, right) => Node::Key(Box::new(substitute(*left, placeholder, replacement)), op, Box::new(substitute(*right, placeholder, replacement))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| substitute(item, placeholder, replacement)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(substitute(*node, placeholder, replacement)), data },
		other => other,
	}
}

fn is_marker(node: &Node, marker: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if name == marker)
}
