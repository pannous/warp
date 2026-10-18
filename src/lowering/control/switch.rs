//! `switch subject {key: body …}` (also `match`): map indexing that executes. Lowered to an if/else chain over the
//! subject, so only the chosen body runs; the `default` (or `_`, `other`, `else`) key catches the rest, without it a miss is the error
//! `no case for <subject> = <value>`. A list key is a structural pattern (wiki/pattern-matching.md): `["", middle, ""]`
//! matches a list of three whose first and last items are "", binding `middle`; `_` matches any item, lists nest, and a
//! pair `k: v` matches a pair whose key is k. A rest `[first, ...rest]` (`*rest`, `rest...`, unnamed `..`) matches a list of at least the other items,
//! binding the rest as a slice; items after it count from the end. A guard `n if n < 0 => …` binds n to the subject and tests the condition; a relational pattern `> 100 => …` compares it. The shape tests are ordinary type tests (`is_type(x, "list") and count(x) == n`).

use super::blocks::{as_data, quoted_data, DATA_WORD};
use super::for_loop::walking_loop;
use super::library_words::COLLECTION_CONTAINS;
use super::nodes::{
	assign, block, call, children_rewritten, fresh_number, if_then, is_kinded_type_word, key, or_operands, Counter,
};
use super::text_patterns::TextPattern;
use super::type_tests::{program_names, type_test, Names};
use crate::analyzer::TEMPORARY_SEPARATOR;
use crate::node::{error, text, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::optional_field;
use crate::warp_parser::subscript;
use crate::words::{COUNT_WORD, LET_WORD, LIST_WORD, MAP_WORD, PAIR_WORD, SLICE, STARRED, STR_WORD, SWITCH_WORDS};

const DEFAULT_KEYS: [&str; 4] = ["default", "other", "else", WILDCARD];
/// In a pattern: any item, bound to no name
pub(crate) const WILDCARD: &str = "_";
const ALTERNATIVES_BIND_NO_NAMES: &str = "alternatives bind no names: write one case per pattern, or `_` for an item";
const KEY_SPEC: &str = "key";
const SUBJECT_PREFIX: &str = "switch_subject_";
/// Pseudo-call `switch_no_case(label: value)` the emitter turns into the runtime error function `no_case_<label>`,
/// which eval reports as `no case for <label> = <value>`
pub const NO_CASE_CALL: &str = "switch_no_case";
pub const NO_CASE_PREFIX: &str = "no_case_";
const UNNAMED_SUBJECT: &str = "value";
/// In an arm: the whole matched value, of the innermost match (user, card match-binding-arm)
const VALUE_WORD: &str = "value";
const ITEM_WORD: &str = "item";
const HELD_WORD: &str = "held";
/// `$a` in a code pattern `data($a + 0)`: a hole that binds the subtree there (card code-holes)
const HOLE_PREFIX: &str = "$";
/// The parts of a key a code pattern reads (wasm_emitter/reflection/node_api.rs)
const OP_FIELD: &str = "op";
const KEY_FIELD: &str = "key";
const VALUE_FIELD: &str = "value";

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&SWITCH_WORDS) {
		return node;
	}
	let context = crate::analyzer::function_definitions(&node);
	let words: Vec<&str> =
		SWITCH_WORDS.into_iter().filter(|word| !context.user_functions.contains_key(*word)).collect();
	if words.is_empty() {
		return node;
	}
	let names = program_names(&node);
	let value_word_free = !names_variable(&node, &context, &names, VALUE_WORD);
	Lowering { words, switches: Counter::default(), names, value_word_free }.expand(node)
}

/// The program has a variable, parameter or loop variable of that name, in any function
fn names_variable(node: &Node, context: &crate::compiler::context::Context, names: &Names, name: &str) -> bool {
	let is_parameter =
		context.user_functions.values().any(|function| function.params.iter().any(|param| param.name == name));
	let mut assigned = std::collections::HashSet::new();
	crate::library_words::collect_assigned_names(node, &mut assigned);
	names.is_variable(name) || is_parameter || assigned.contains(name)
}

struct Lowering<'a> {
	words: Vec<&'a str>,
	switches: Counter,
	/// The program's variables and declared types: a type word in a case tests the subject's type (`int => …`)
	names: Names,
	/// No variable, parameter or loop variable of the program is named value: in an arm it is the matched value
	value_word_free: bool,
}

struct Case {
	key: Node,
	body: Node,
}

impl Lowering<'_> {
	fn expand(&self, node: Node) -> Node {
		match node {
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				match self.switch(&items) {
					Some(switch) => switch,
					None => Node::List(self.with_switch_arguments(items), bracket, separator),
				}
			}
			other => children_rewritten(other, |child| self.expand(child)),
		}
	}

	/// `f(switch 2 {1: 10 2: 20})` arrives as the call `f switch 2 {…}`: the three items one argument
	fn with_switch_arguments(&self, items: Vec<Node>) -> Vec<Node> {
		let Some(start) =
			(1..items.len().saturating_sub(2)).find(|&start| self.switch(&items[start..start + 3]).is_some())
		else {
			return items;
		};
		let switch = self.switch(&items[start..start + 3]).expect("found");
		let rest = self.with_switch_arguments(items[start + 3..].to_vec());
		items[..start].iter().cloned().chain([switch]).chain(rest).collect()
	}

	/// `switch subject {cases}`; `switch 3 {…} == "three"` arrives as `switch 3 ({…} == "three")`: the switch's value
	/// is the left operand
	fn switch(&self, items: &[Node]) -> Option<Node> {
		let (head, subject, cases) = split_switch(items)?;
		if let Node::Key(..) = cases.drop_meta() {
			let block = leftmost_operand(cases);
			let switch = self.switch(&[head.clone(), subject.clone(), block.clone()])?;
			return Some(with_leftmost_operand(cases.clone(), switch));
		}
		let word = head.symbol_name()?;
		if !self.words.contains(&word) {
			return None;
		}
		let number = self.switches.next_number();
		let subject_name = format!("{SUBJECT_PREFIX}{number}");
		let subject_value = Node::Symbol(subject_name.clone());
		let cases = cases_of(cases)?;
		let cases = match self.value_word_free {
			true => cases
				.into_iter()
				.map(|case| Case {
					key: as_subject(case.key, &subject_value),
					body: as_subject(case.body, &subject_value),
				})
				.collect(),
			false => cases,
		};
		let (default, cases): (Vec<Case>, Vec<Case>) = cases.into_iter().partition(|case| is_default(&case.key));
		let miss = match default.into_iter().next() {
			Some(default) => default.body,
			None => no_case(subject, &subject_value),
		};
		let chain = cases.into_iter().rev().fold(miss, |otherwise, case| {
			let (matches, body) = case_test(&subject_value, case, &self.names);
			let then = if_then(matches, body);
			key(then, Op::Else, otherwise)
		});
		let bind = assign(subject_value, subject.clone());
		Some(Node::List(vec![bind, chain], Bracket::Round, Separator::Semicolon))
	}
}

/// The word value in an arm as the subject; a field `x.value` and a key `{value: 1}` keep it
fn as_subject(node: Node, subject: &Node) -> Node {
	match node {
		Node::Symbol(name) if name == VALUE_WORD => subject.clone(),
		Node::Key(left, Op::Dot, right) => key(as_subject(*left, subject), Op::Dot, *right),
		Node::Key(left, Op::Colon, right) => key(*left, Op::Colon, as_subject(*right, subject)),
		other => children_rewritten(other, |child| as_subject(child, subject)),
	}
}

/// The word, subject and case block of a switch as the parser groups them: the items [switch, subject, {cases}],
/// the pair [(switch subject), {cases}], or, when the block juxtaposes with a variable subject, [switch, (subject {cases})]
pub(crate) fn split_switch(items: &[Node]) -> Option<(&Node, &Node, &Node)> {
	match items {
		[head, subject, cases] => Some((head, subject, cases)),
		[pair, cases] if matches!(pair.drop_meta(), Node::List(words, _, _) if words.len() == 2) => {
			match pair.drop_meta() {
				Node::List(words, _, _) => Some((&words[0], &words[1], cases)),
				_ => None,
			}
		}
		[head, juxtaposed] => match juxtaposed.drop_meta() {
			Node::List(pair, _, _) if pair.len() == 2 => Some((head, &pair[0], &pair[1])),
			_ => None,
		},
		_ => None,
	}
}

/// `match`, `switch`: a word that opens a switch
pub(crate) fn is_switch_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::Symbol(word) if SWITCH_WORDS.contains(&word.as_str()))
}

/// The leftmost operand of an operator chain
fn leftmost_operand(node: &Node) -> &Node {
	match node.drop_meta() {
		Node::Key(left, _, _) => leftmost_operand(left),
		_ => node,
	}
}

fn with_leftmost_operand(node: Node, replacement: Node) -> Node {
	match node {
		Node::Key(left, op, right) => Node::Key(Box::new(with_leftmost_operand(*left, replacement)), op, right),
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_leftmost_operand(node, replacement)),
		_ => replacement,
	}
}

fn is_default(key: &Node) -> bool {
	matches!(key.drop_meta(), Node::Symbol(name) if DEFAULT_KEYS.contains(&name.as_str()))
}

/// The item at 1-based `position` of the value at `path`: `subject#2`, `subject#2#1`
fn item(path: &Node, position: usize) -> Node {
	key(path.clone(), Op::Hash, Node::int(position as i64))
}

/// A rest in a pattern, with its name: `...rest` and `*rest` (parsed alike), `rest...`; `..` and `...` unnamed
pub(crate) fn rest_pattern(part: &Node) -> Option<Option<String>> {
	match part.drop_meta() {
		Node::Symbol(name) => {
			name.strip_prefix(STARRED).filter(|rest| !rest.is_empty()).map(|rest| Some(rest.to_string()))
		}
		Node::Key(start, Op::Range | Op::To, end) if end.is_empty_node() => match start.drop_meta() {
			Node::Empty => Some(None),
			Node::Symbol(name) => Some(Some(name.clone())),
			_ => None,
		},
		_ => None,
	}
}

/// An item pattern `[a, b]...` (also `{sku}...`) in a list pattern's rest position: every item of the rest matches it
fn item_pattern(part: &Node) -> Option<&Node> {
	match part.drop_meta() {
		Node::Key(each, Op::Range | Op::To, end)
			if end.is_empty_node() && matches!(each.drop_meta(), Node::List(..)) =>
		{
			Some(each)
		}
		_ => None,
	}
}

fn is_rest(part: &Node) -> bool {
	rest_pattern(part).is_some() || item_pattern(part).is_some()
}

/// `[[n, s]...]`: a loop testing every item against the item pattern, appending the value of each name it binds to that
/// name's list; the test holds when every item matched, the names are bound to the lists (card list-pattern)
fn every_item_test(each: &Node, items: Node, bindings: &mut Vec<Node>, names: &Names) -> Node {
	let number = fresh_number();
	let temporary = |name: &str| Node::Symbol(format!("{name}{TEMPORARY_SEPARATOR}{number}"));
	let (item, held) = (temporary(ITEM_WORD), temporary(HELD_WORD));
	let Matched { tests: item_tests, bindings: item_bindings, .. } = Matched::of(each, item.clone(), names);
	let mut statements = vec![assign(held.clone(), Node::True)];
	let mut appends = vec![];
	for (name, value) in item_bindings.iter().filter_map(bound) {
		let list = temporary(&name.name());
		statements.push(assign(list.clone(), Node::Empty));
		appends.push(key(
			list.clone(),
			Op::AddAssign,
			Node::List(vec![value.clone()], Bracket::Square, Separator::None),
		));
		bindings.push(bind(name.clone(), list));
	}
	let missed = assign(held.clone(), Node::False);
	let verdict = match appends.is_empty() {
		true => if_then(key(Node::Empty, Op::Not, joined(item_tests, Op::And)), missed),
		false => key(if_then(joined(item_tests, Op::And), block(appends)), Op::Else, missed),
	};
	statements.push(walking_loop(&item, items, vec![verdict]));
	statements.push(held);
	semicolon_sequence(statements)
}

/// The names a pattern binds, in order
pub(crate) fn pattern_bindings(pattern: &Node) -> Vec<Node> {
	Matched::of(pattern, Node::Empty, &Names::default())
		.bindings
		.iter()
		.filter_map(bound)
		.map(|(name, _)| name.clone())
		.collect()
}

/// The name and value of a binding `let name = value`
fn bound(binding: &Node) -> Option<(&Node, &Node)> {
	match binding.drop_meta() {
		Node::List(parts, _, _) => match parts.as_slice() {
			[_, declaration] => match declaration.drop_meta() {
				Node::Key(name, Op::Assign, value) => Some((name.as_ref(), value.as_ref())),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

/// `{name: n, body: {user}}` matches a map partially (card map-patterns): only fields, each a name, `name: pattern`
/// or an unnamed rest `..`
pub(crate) fn is_map_pattern(pattern: &Node) -> bool {
	match pattern.drop_meta() {
		Node::List(fields, Bracket::Curly, _) => {
			!fields.is_empty()
				&& fields.iter().all(|part| map_field(part).is_some() || rest_pattern(part) == Some(None))
		}
		_ => false,
	}
}

/// A field of a map pattern as its name and its pattern: `{name}` binds the field to its own name; `debug?: bool` is
/// the optional field (card schemas)
pub(crate) fn map_field(part: &Node) -> Option<(String, &Node)> {
	if let Some(field) = optional_field(part) {
		return map_field(field);
	}
	match part.drop_meta() {
		Node::Symbol(name) if rest_pattern(part).is_none() => Some((name.clone(), part)),
		Node::Key(name, Op::Colon, pattern) if matches!(name.drop_meta(), Node::Symbol(_) | Node::Text(_)) => {
			Some((name.name(), pattern))
		}
		_ => None,
	}
}

/// `[a, b]` and the tuple `(a, b)` (card match-tuples) match by shape; a parenthesised single item `(x)` is a group
pub(crate) fn is_list_pattern(pattern: &Node) -> bool {
	match pattern.drop_meta() {
		Node::List(_, Bracket::Square, _) => true,
		Node::List(items, Bracket::Round, Separator::Comma) => items.len() > 1,
		_ => false,
	}
}

/// What a pattern asks of the value it matches: the tests it passes, and the assignments of the names it binds
struct Matched<'a> {
	names: &'a Names,
	tests: Vec<Node>,
	bindings: Vec<Node>,
}

impl<'a> Matched<'a> {
	fn of(pattern: &Node, path: Node, names: &'a Names) -> Self {
		let mut matched = Matched { names, tests: vec![], bindings: vec![] };
		matched.pattern(pattern, path);
		matched
	}

	/// The tests and bindings of `pattern` matching the value at `path`; at any depth a part takes what a whole arm
	/// takes (card deep-conditions)
	fn pattern(&mut self, pattern: &Node, path: Node) {
		if let Some(code) = code_pattern(pattern) {
			return code_tests(&code, path, &mut self.tests, &mut self.bindings);
		}
		if let Some(text_pattern) = TextPattern::of(pattern, &|name| self.names.contains(name)) {
			return text_pattern.matched(&path, &mut self.tests, &mut self.bindings);
		}
		if let Some(test) = condition(&path, pattern, self.names) {
			self.tests.push(test);
			return;
		}
		match pattern.drop_meta() {
			Node::List(items, _, _) if is_list_pattern(pattern) => self.items(items, path),
			// fields not named are ignored, the named ones must be there
			Node::List(fields, _, _) if is_map_pattern(pattern) => {
				// a map of one field is a pair
				let is = |spec| call(crate::words::IS_TYPE, vec![path.clone(), text(spec)]);
				self.tests.push(key(is(MAP_WORD), Op::Or, is(PAIR_WORD)));
				for field_part in fields {
					let Some((name, part)) = map_field(field_part) else {
						continue;
					};
					let present = call(COLLECTION_CONTAINS, vec![path.clone(), Node::Text(name.clone())]);
					let field = subscript(path.clone(), Node::Text(name.clone()));
					if optional_field(field_part).is_some() {
						// missing, or there and matching
						let Matched { tests, bindings, .. } = Matched::of(part, field, self.names);
						let missing = key(Node::Empty, Op::Not, present);
						self.tests.push(key(missing, Op::Or, joined(tests, Op::And)));
						self.bindings.extend(bindings);
						continue;
					}
					self.tests.push(present);
					match part.drop_meta() {
						Node::Symbol(shorthand) if *shorthand == name => self.bind(part, field), // `{text}` too
						_ => self.pattern(part, field),
					}
				}
			}
			// `a: x` matches the pair whose key is a (a name, never bound) and matches its value against x
			Node::Key(name, Op::Colon, value) => {
				self.tests.push(call(crate::words::IS_TYPE, vec![path.clone(), text(PAIR_WORD)]));
				self.tests.push(key(call(STR_WORD, vec![item(&path, 1)]), Op::Eq, Node::Text(name.name())));
				self.pattern(value, item(&path, 2));
			}
			Node::Symbol(name) if name == WILDCARD => {}
			Node::Symbol(_) => self.bind(pattern, path),
			literal => self.tests.push(key(path, Op::Eq, literal.clone())),
		}
	}

	fn bind(&mut self, name: &Node, path: Node) {
		self.bindings.push(bind(name.drop_meta().clone(), path));
	}

	/// `[a, b]` by count and position; `[first, ...middle, last]` a slice for the rest
	fn items(&mut self, items: &[Node], path: Node) {
		for (part, part_path) in items.iter().zip(item_paths(items, &path, is_rest, &mut self.tests)) {
			if let Some(each) = item_pattern(part) {
				let test = every_item_test(each, part_path, &mut self.bindings, self.names);
				self.tests.push(test);
			} else if let Some(rest) = rest_pattern(part) {
				if let Some(name) = rest {
					self.bind(&Node::Symbol(name), part_path);
				}
			} else {
				self.pattern(part, part_path);
			}
		}
	}
}

/// The count tests of a list of `items` at `path`, and the path of each item: before a rest from the start, after it
/// from the end, the rest its slice
fn item_paths(items: &[Node], path: &Node, is_rest: impl Fn(&Node) -> bool, tests: &mut Vec<Node>) -> Vec<Node> {
	tests.push(type_named(path, LIST_WORD));
	let count = call(COUNT_WORD, vec![path.clone()]);
	let Some(rest_index) = items.iter().position(is_rest) else {
		tests.push(key(count, Op::Eq, Node::int(items.len() as i64)));
		return (1..=items.len()).map(|position| item(path, position)).collect();
	};
	let fixed = items.len() - 1;
	tests.push(key(count.clone(), Op::Ge, Node::int(fixed as i64)));
	let from_end = |distance: usize| match distance {
		0 => count.clone(),
		_ => key(count.clone(), Op::Sub, Node::int(distance as i64)),
	};
	(0..items.len())
		.map(|index| match index.cmp(&rest_index) {
			std::cmp::Ordering::Less => item(path, index + 1),
			std::cmp::Ordering::Greater => key(path.clone(), Op::Hash, from_end(fixed - index)),
			std::cmp::Ordering::Equal => {
				call(SLICE, vec![path.clone(), Node::int(rest_index as i64), from_end(fixed - rest_index)])
			}
		})
		.collect()
}

/// A test of the value at `path` that binds nothing: a comparison `>= 18`, a type word (`int`, `Circle`), a range
/// `13 to 17` or alternatives `1 | 2`
fn condition(path: &Node, pattern: &Node, names: &Names) -> Option<Node> {
	if let Node::Key(empty, op, bound) = pattern.drop_meta() {
		if empty.is_empty_node() && op.is_ordering() {
			return Some(key(path.clone(), *op, bound.as_ref().clone()));
		}
	}
	if let Some(test) = type_test(path, pattern, names) {
		return Some(test);
	}
	if let Some(tests) = range_tests(path, pattern) {
		return Some(joined(tests, Op::And));
	}
	let alternatives = or_operands(pattern);
	if alternatives.len() < 2 {
		return None;
	}
	let tests = alternatives.iter().map(|alternative| {
		let matched = Matched::of(alternative, path.clone(), names);
		match matched.bindings.is_empty() {
			true => joined(matched.tests, Op::And),
			false => error(ALTERNATIVES_BIND_NO_NAMES),
		}
	});
	Some(joined(tests.collect(), Op::Or))
}

/// `data($a + 0)`: quoted code holding a hole
pub(crate) fn code_pattern(pattern: &Node) -> Option<Node> {
	let code = match pattern.drop_meta() {
		// `data $f ($x)`: the words after data
		Node::List(words, Bracket::None, Separator::Space) if words.len() > 2 && words[0].is_symbol(DATA_WORD) => {
			Node::List(words[1..].to_vec(), Bracket::None, Separator::Space)
		}
		_ => quoted_data(pattern)?.clone(),
	};
	has_hole(&code).then_some(code)
}

/// `$a` in quoted code: a, the name it binds in a pattern and splices in a template
pub(crate) fn hole_name(part: &Node) -> Option<&str> {
	part.drop_meta().symbol_name()?.strip_prefix(HOLE_PREFIX).filter(|name| !name.is_empty())
}

fn has_hole(code: &Node) -> bool {
	match code.drop_meta() {
		Node::Key(left, _, right) => has_hole(left) || has_hole(right),
		Node::List(items, _, _) => items.iter().any(has_hole),
		part => hole_name(part).is_some(),
	}
}

/// The code at `path` has the shape of the quoted code: a hole binds the subtree there, a repeated hole must be the same
/// subtree, a key has the same operator and a list the same length, parts without holes are equal as data
fn code_tests(code: &Node, path: Node, tests: &mut Vec<Node>, bindings: &mut Vec<Node>) {
	if let Some(name) = hole_name(code) {
		return bind_hole(name, path, tests, bindings);
	}
	match code.drop_meta() {
		Node::Key(left, op, right) if has_hole(code) => {
			let part = |field: &str| key(path.clone(), Op::Dot, Node::Symbol(field.to_string()));
			tests.push(type_named(&path, KEY_SPEC));
			tests.push(key(part(OP_FIELD), Op::Eq, text(op.as_str())));
			code_tests(left, part(KEY_FIELD), tests, bindings);
			code_tests(right, part(VALUE_FIELD), tests, bindings);
		}
		Node::List(items, _, _) if has_hole(code) => {
			let items = called_hole_items(items);
			let is_rest = |part: &Node| rest_hole(part).is_some();
			for (part, part_path) in items.iter().zip(item_paths(&items, &path, is_rest, tests)) {
				match rest_hole(part) {
					Some(name) => bind_hole(name, part_path, tests, bindings),
					None => code_tests(part, part_path, tests, bindings),
				}
			}
		}
		_ => tests.push(key(path, Op::Eq, as_data(code.clone()))),
	}
}

/// A hole binds the subtree at `path`; a repeated hole must be the same subtree
fn bind_hole(name: &str, path: Node, tests: &mut Vec<Node>, bindings: &mut Vec<Node>) {
	let name = Node::Symbol(name.to_string());
	match bindings.iter().filter_map(bound).find(|(bound_name, _)| **bound_name == name) {
		Some((_, earlier)) => tests.push(key(path, Op::Eq, earlier.clone())),
		None => bindings.push(bind(name, path)),
	}
}

/// `$xs...` in quoted code: the hole xs for the items from there on
pub(crate) fn rest_hole(part: &Node) -> Option<&str> {
	match part.drop_meta() {
		Node::Key(start, Op::Range | Op::To, end) if end.is_empty_node() => hole_name(start),
		_ => None,
	}
}

/// `$f($x, $y)` reads as the hole `$f` and the group `($x, $y)`, where a call `g(x, y)` of a name quotes as `g x y`
fn called_hole_items(items: &[Node]) -> Vec<Node> {
	hole_call(items).unwrap_or_else(|| items.to_vec())
}

/// `$f($x, $y)`: the call's items `$f $x $y`
pub(crate) fn hole_call(items: &[Node]) -> Option<Vec<Node>> {
	match items {
		[head, arguments] if hole_name(head).is_some() => match arguments.drop_meta() {
			Node::List(arguments, Bracket::Round, _) => Some([vec![head.clone()], arguments.clone()].concat()),
			_ => None,
		},
		_ => None,
	}
}

fn type_named(path: &Node, type_word: &str) -> Node {
	call(crate::words::IS_TYPE, vec![path.clone(), text(type_word)])
}

/// A case as its test and its body: a list key is a structural pattern whose names are bound before the body runs; a
/// guard `n if n < 0` binds n to the subject and holds when its condition does
fn case_test(subject: &Node, case: Case, names: &Names) -> (Node, Node) {
	let (pattern, guard) = split_guard(&case.key);
	let binds = guard.is_some() || is_binding_name(&pattern, &case.body, names);
	let (test, bindings) = match pattern_test(subject, &pattern, binds, names) {
		Ok(matched) => matched,
		Err(message) => return (error(message), case.body),
	};
	let test = match guard {
		Some(guard) => {
			let guard = semicolon_sequence([bindings.clone(), vec![guard]].concat());
			match test {
				Node::True => guard, // a class pattern `Circle(r)` arrives as `_` guarded by its type test
				test => key(test, Op::And, guard),
			}
		}
		None => test,
	};
	let body = if bindings.is_empty() { case.body } else { semicolon_sequence([bindings, vec![case.body]].concat()) };
	(test, body)
}

/// `name => "hi " + name`: a name defined nowhere that its body uses binds the subject (card match-binding-arm); unused,
/// `red: "fire"` is the symbol red
fn is_binding_name(pattern: &Node, body: &Node, names: &Names) -> bool {
	match pattern.drop_meta() {
		Node::Symbol(name) => {
			!names.contains(name) && !is_kinded_type_word(pattern) && body.mentions_any(&[name.as_str()])
		}
		_ => false,
	}
}

/// The test the subject passes when it matches the pattern of a case, and the names the pattern binds. A case key is a
/// pattern, so `1 | 2` and `1 or 2` (both parsed as or) are its alternatives, never the logical value; a computed
/// comparison goes in a guard (user, matching roadmap)
pub(crate) fn pattern_test(
	subject: &Node,
	pattern: &Node,
	guarded: bool,
	names: &Names,
) -> Result<(Node, Vec<Node>), &'static str> {
	let alternatives = or_operands(pattern);
	if let [single] = alternatives.as_slice() {
		let (tests, bindings) = alternative_tests(subject, single, guarded, names);
		return Ok((joined(tests, Op::And), bindings));
	}
	let mut matches = vec![];
	for alternative in &alternatives {
		let (tests, bindings) = alternative_tests(subject, alternative, guarded, names);
		if !bindings.is_empty() {
			return Err(ALTERNATIVES_BIND_NO_NAMES);
		}
		matches.push(joined(tests, Op::And));
	}
	Ok((joined(matches, Op::Or), vec![]))
}

/// `xs is [_, 2]` and `if xs is [a, b] then b` (card is-pattern): a list or tuple holding `_`, a rest or a name defined
/// nowhere is a pattern, its names bound as the test holds; a list of known values stays a comparison
pub(crate) fn is_pattern(subject: &Node, pattern: &Node, names: &Names) -> Option<Node> {
	let is_structural = (is_list_pattern(pattern) || is_map_pattern(pattern)) && has_open_part(pattern, names);
	if !is_structural
		&& code_pattern(pattern).is_none()
		&& TextPattern::of(pattern, &|name| names.contains(name)).is_none()
	{
		return None;
	}
	let Matched { mut tests, bindings, .. } = Matched::of(pattern, subject.clone(), names);
	if !bindings.is_empty() {
		tests.push(semicolon_sequence([bindings, vec![Node::True]].concat()));
	}
	Some(joined(tests, Op::And))
}

/// `_`, a rest, a comparison `>= 18` or a name not defined anywhere among the parts of a pattern: what a value never is
fn has_open_part(pattern: &Node, names: &Names) -> bool {
	if is_rest(pattern) {
		return true;
	}
	match pattern.drop_meta() {
		Node::Key(empty, op, _) => empty.is_empty_node() && op.is_ordering(),
		Node::List(items, _, _) if is_list_pattern(pattern) => items.iter().any(|part| has_open_part(part, names)),
		// a schema `{port: int, debug?: bool}`: no map holds a type or an optional field
		Node::List(fields, _, _) if is_map_pattern(pattern) => fields.iter().any(|part| {
			optional_field(part).is_some()
				|| map_field(part).is_some_and(|(_, part)| is_kinded_type_word(part) || has_open_part(part, names))
		}),
		Node::Symbol(name) => name == WILDCARD || !names.contains(name) && !is_kinded_type_word(pattern),
		_ => false,
	}
}

/// A name a pattern binds is declared `let`: the arm's own local, never a change of a main-level variable of that name
/// (card pattern-names)
pub(crate) fn bind(name: Node, value: Node) -> Node {
	Node::List(vec![Node::Symbol(LET_WORD.to_string()), assign(name, value)], Bracket::None, Separator::Space)
}

/// A list pattern tests its shape, a type word (`int`, `Circle`) the subject's type, `i: int` also binds it; any other
/// key compares the subject, a guarded name binds it
fn alternative_tests(subject: &Node, pattern: &Node, guarded: bool, names: &Names) -> (Vec<Node>, Vec<Node>) {
	if let Node::Key(name, Op::Colon, kind) = pattern.drop_meta() {
		if let (Node::Symbol(_), Some(test)) = (name.drop_meta(), type_test(subject, kind, names)) {
			return (vec![test], vec![bind(name.drop_meta().clone(), subject.clone())]);
		}
	}
	if let Some(test) = condition(subject, pattern, names) {
		return (vec![test], vec![]);
	}
	let is_structural = is_list_pattern(pattern) || is_map_pattern(pattern) || code_pattern(pattern).is_some();
	if !guarded && !is_structural && TextPattern::of(pattern, &|name| names.contains(name)).is_none() {
		return (vec![key(subject.clone(), Op::Eq, pattern.clone())], vec![]);
	}
	let matched = Matched::of(pattern, subject.clone(), names);
	(matched.tests, matched.bindings)
}

/// `13 to 19` (also `13...19`) includes its end, `13..20` excludes it, `65..` has none: the subject between the bounds
fn range_tests(subject: &Node, pattern: &Node) -> Option<Vec<Node>> {
	let Node::Key(start, op @ (Op::Range | Op::To), end) = pattern.drop_meta() else {
		return None;
	};
	let below_end = if *op == Op::To { Op::Le } else { Op::Lt };
	let bounds = [(start, Op::Ge), (end, below_end)];
	let tests: Vec<Node> = bounds
		.into_iter()
		.filter(|(bound, _)| !bound.is_empty_node())
		.map(|(bound, comparison)| key(subject.clone(), comparison, bound.as_ref().clone()))
		.collect();
	(!tests.is_empty()).then_some(tests)
}

/// The tests joined by `and` (true without any) or by `or`
fn joined(tests: Vec<Node>, op: Op) -> Node {
	tests.into_iter().reduce(|all, test| key(all, op, test)).unwrap_or(Node::True)
}

/// `n if n < 0` (parsed as `if n < 0 then n`): the pattern n and the guard `n < 0`; any other key has no guard
fn split_guard(case_key: &Node) -> (Node, Option<Node>) {
	match case_key.drop_meta() {
		Node::Key(condition, Op::Then, pattern) => match condition.drop_meta() {
			Node::Key(empty, Op::If, guard) if empty.is_empty_node() => {
				(pattern.drop_meta().clone(), Some(guard.as_ref().clone()))
			}
			_ => (case_key.clone(), None),
		},
		_ => (case_key.clone(), None),
	}
}

fn semicolon_sequence(statements: Vec<Node>) -> Node {
	match <[Node; 1]>::try_from(statements) {
		Ok([single]) => single,
		Err(statements) => Node::List(statements, Bracket::Round, Separator::Semicolon),
	}
}

/// What the error names: a literal or a variable subject as written, anything else just "value"
fn subject_label(subject: &Node) -> String {
	let written = match subject.drop_meta() {
		Node::Number(number) => number.to_string(),
		Node::Text(text) | Node::Symbol(text) => text.clone(),
		Node::Char(letter) => letter.to_string(),
		_ => String::new(),
	};
	let is_name = !written.is_empty() && written.chars().all(|c| c.is_alphanumeric() || c == '_');
	if is_name {
		written
	} else {
		UNNAMED_SUBJECT.to_string()
	}
}

/// `switch_no_case("label": value)`: the error names the subject as written and its runtime value
fn no_case(subject: &Node, value: &Node) -> Node {
	let label_and_value = key(Node::Text(subject_label(subject)), Op::Colon, value.clone());
	call(NO_CASE_CALL, vec![label_and_value])
}

/// The `key: body` entries of a block
fn cases_of(block: &Node) -> Option<Vec<Case>> {
	let single;
	let items: &[Node] = match block.drop_meta() {
		Node::List(items, Bracket::Curly, _) if !items.is_empty() => items,
		Node::Key(..) => {
			single = [block.clone()];
			&single
		}
		_ => return None,
	};
	// `1, 2 => …` parses as the items 1 and `2 => …`: the plain items before a case are its alternatives
	let mut cases = vec![];
	let mut alternatives: Vec<Node> = vec![];
	for item in items.iter().flat_map(comma_items) {
		match case_of(&item) {
			Some(case) => {
				let key = alternatives.drain(..).rev().fold(case.key, |later, earlier| key(earlier, Op::Or, later));
				cases.push(Case { key, body: case.body });
			}
			None => alternatives.push(item),
		}
	}
	alternatives.is_empty().then_some(cases)
}

/// The items of a comma list inside a case block, else the item itself
fn comma_items(item: &Node) -> Vec<Node> {
	match item.drop_meta() {
		Node::List(items, Bracket::None, Separator::Comma) => items.clone(),
		_ => vec![item.clone()],
	}
}

/// `1: 10` is a Colon key; `1: x=5` parses as `(1:x)=5`, so the colon is found down the left spine of the assignment
fn case_of(item: &Node) -> Option<Case> {
	match item.drop_meta() {
		Node::Key(key, Op::Colon, body) => Some(Case { key: key.as_ref().clone(), body: body.as_ref().clone() }),
		Node::Key(left, op, right) => {
			let Case { key, body } = case_of(left)?;
			Some(Case { key, body: Node::Key(Box::new(body), *op, right.clone()) })
		}
		_ => None,
	}
}
