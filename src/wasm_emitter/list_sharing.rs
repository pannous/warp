//! Which typed lists may stay arrays now that lists are shared (P200b, card shared-lists): a typed list is held by
//! its variables alone. Wherever the program needs it as a Node (an argument, an item, a variable holding Nodes) that
//! Node is a copy, so the two would part as soon as either changes: such a list stays the Node list every holder shares.
//! Reading it as a whole (`count xs`, `print xs`, a function that never changes its parameter) is no holding.

use crate::analyzer::is_list_mutating_method;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::map_backend::is_update;
use crate::context::UserFunctionDef;

/// Words that read a list without holding on to it, besides those reading any collection (reads_collection)
const LIST_READING_WORDS: [&str; 4] = ["join", "sum", crate::library_words::LIST_SUM, super::list_abi::RETURN];

/// The variables `program` hands on as a whole Node: an argument of a function that changes that parameter (or of an
/// imported one that `changes_arguments` may), an item of a list or map, a field value (not the program's final
/// variable, a value read or discarded)
pub(super) fn held_elsewhere<'a>(program: &'a Node, functions: &BTreeMap<String, UserFunctionDef>, changes_arguments: &dyn Fn(&str) -> bool) -> Holds<'a> {
	let mut walk = Holders { functions, changes_arguments, passing: false, holds: Holds::default() };
	walk.value(program, true);
	walk.holds
}

#[derive(Default)]
pub(super) struct Holds<'n> {
	/// The places an item or a field holds each variable: `[xs, 5]`, `{r = xs}`
	pub places: HashMap<String, Vec<&'n Node>>,
	/// The variables passed to a function that changes them
	pub passed: HashSet<String>,
}

struct Holders<'a, 'n> {
	functions: &'a BTreeMap<String, UserFunctionDef>,
	changes_arguments: &'a dyn Fn(&str) -> bool,
	/// inside an argument a function may change
	passing: bool,
	holds: Holds<'n>,
}

impl<'n> Holders<'_, 'n> {
	/// The variables a value holds; `read`: where it goes only reads it (a reading word, a discarded statement, the
	/// result), so the variable it is stays unheld
	fn value(&mut self, node: &'n Node, read: bool) {
		let place = node.drop_meta();
		match place {
			Node::Symbol(name) if !read && self.passing => { self.holds.passed.insert(name.clone()); }
			Node::Symbol(name) if !read => self.holds.places.entry(name.clone()).or_default().push(place),
			Node::Symbol(_) => {}
			Node::List(statements, _, Separator::Semicolon | Separator::Newline) => {
				for (index, statement) in statements.iter().enumerate() {
					self.value(statement, read || index + 1 < statements.len());
				}
			}
			Node::Key(target, op, value) if matches!(op, Op::Assign | Op::Define) || is_update(op) => self.assignment(target, value),
			// `xs#i`, `#xs`: an item or the count read
			Node::Key(list, Op::Hash, index) => { self.value(list, true); self.value(index, true) }
			Node::Key(left, op, right) if op.is_comparison() => { self.value(left, true); self.value(right, true) }
			Node::Key(object, Op::Dot, member) if is_reading(member) => self.value(object, true),
			Node::Key(left, _, right) => { self.value(left, false); self.value(right, false) }
			Node::List(items, Bracket::Round, Separator::None) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => {
				let callee = items[0].drop_meta().name();
				let function = self.functions.get(&callee);
				let imported_changing = function.is_none() && (self.changes_arguments)(&callee);
				for (index, argument) in items.iter().enumerate().skip(1) {
					// a function that never changes the list it gets only reads it
					let changed = imported_changing || function.is_some_and(|function| changes_parameter(function, index - 1));
					let passing = std::mem::replace(&mut self.passing, changed);
					self.value(argument, !changed);
					self.passing = passing;
				}
			}
			// `(x)`: the value itself
			Node::List(items, Bracket::Round, _) if items.len() == 1 => self.value(&items[0], read),
			Node::List(items, _, _) if items.first().is_some_and(is_reading) => items.iter().skip(1).for_each(|item| self.value(item, true)),
			Node::List(items, _, _) => items.iter().for_each(|item| self.value(item, false)),
			_ => {}
		}
	}

	fn assignment(&mut self, target: &'n Node, value: &'n Node) {
		match (target.drop_meta(), value.drop_meta()) {
			// `xs = xs + [v]`: only the new items
			(Node::Symbol(name), Node::Key(list, Op::Add, added)) if matches!(list.drop_meta(), Node::Symbol(list) if list == name) => self.value(added, false),
			// `ys = xs`, `ys = (…; xs)`: one list in two variables, which find_typed_lists types together or not at all
			(Node::Symbol(_), _) => self.value(value, true),
			(Node::Key(list, Op::Hash, index), _) if matches!(list.drop_meta(), Node::Symbol(_)) => { self.value(index, true); self.value(value, false) }
			(other, _) => { self.value(other, false); self.value(value, false) }
		}
	}
}

/// The calls that change the list they are given: what `xs.pop()`, `xs.insert(…)` and `xs.remove(v)` lower to
const IN_PLACE_CALLS: [&str; 4] = [crate::analyzer::LIST_DROP_LAST, crate::analyzer::INSERT_AT_CALL, crate::analyzer::INSERT_EITHER_CALL,
	crate::library_words::MAP_WITHOUT];

/// `list_drop_last(xs)`, `list_insert_at(xs, i, v)`, `map_without(xs, v)`: a call that changes the list it is given
pub(super) fn changes_in_place(value: &Node) -> bool {
	matches!(super::list_abi::called_function(value), Some(name) if IN_PLACE_CALLS.contains(&name))
}

fn is_reading(word: &Node) -> bool {
	matches!(word.drop_meta(), Node::Symbol(word) if super::reads_collection(word) || LIST_READING_WORDS.contains(&word.as_str()))
}

/// Does the function change the list its parameter `index` holds: an item set, an append, a method that changes it
fn changes_parameter(function: &UserFunctionDef, index: usize) -> bool {
	function.params.get(index).is_some_and(|param| changes_list(&function.body, &param.name))
}

/// Does `body` change the list `name` holds in place: `name#i = v`, `name += …`, `name.add(v)`, `name.pop()`
pub fn changes_list(body: &Node, name: &str) -> bool {
	let is_name = |node: &Node| matches!(node.drop_meta(), Node::Symbol(symbol) if symbol == name);
	let mut found = false;
	body.visit(&mut |part| match part {
		Node::Key(target, op, value) if matches!(op, Op::Assign | Op::Define) || is_update(op) => match target.drop_meta() {
			Node::Key(list, Op::Hash, _) => found |= is_name(list),
			target => found |= is_name(target) && (is_update(op) || changes_in_place(value)),
		},
		Node::Key(list, Op::Dot, call) if is_name(list) => {
			found |= matches!(call.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(method)) if is_list_mutating_method(method)));
		}
		_ => {}
	});
	found
}

/// The variables whose lists `body` changes in place (an item set, an append, a method that changes it, an argument to
/// a function that changes it), each by the variable it is reached from: `grid#i#j = v` changes grid. A list reached
/// otherwise (`f()#1 = v`), or a function that changes any list, is the unknown root ""
pub(super) fn written_roots(body: &Node, functions: &BTreeMap<String, UserFunctionDef>) -> HashSet<String> {
	let mut roots = HashSet::new();
	body.visit(&mut |part| match part {
		Node::Key(target, op, value) if matches!(op, Op::Assign | Op::Define) || is_update(op) => match target.drop_meta() {
			Node::Key(list, Op::Hash, _) => { roots.insert(root(list)); }
			Node::Symbol(name) if is_update(op) || changes_in_place(value) => { roots.insert(name.clone()); }
			_ => {}
		},
		Node::Key(list, Op::Dot, call) if matches!(call.drop_meta(), Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(method)) if is_list_mutating_method(method))) => {
			roots.insert(root(list));
		}
		Node::List(items, Bracket::Round, Separator::None) => {
			let Some(function) = items.first().and_then(|callee| functions.get(&callee.drop_meta().name())) else { return };
			for (index, argument) in items.iter().enumerate().skip(1) {
				if changes_parameter(function, index - 1) {
					roots.insert(root(argument));
				}
			}
			// a list it reaches otherwise (a global, a field of an argument)
			let own = written_roots(&function.body, &BTreeMap::new());
			if own.iter().any(|written| !function.params.iter().any(|param| param.name == *written)) {
				roots.insert(String::new());
			}
		}
		_ => {}
	});
	roots
}

/// The variable a place is reached from: `grid` of `grid#i#j`, `p` of `p.xs`; "" for any other
fn root(place: &Node) -> String {
	match place.drop_meta() {
		Node::Symbol(name) => name.clone(),
		Node::Key(base, Op::Hash | Op::Dot, _) => root(base),
		_ => String::new(),
	}
}
