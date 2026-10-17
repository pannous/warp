//! List and map mutation methods: `xs.add(v)`, `xs.pop()`, `m.remove(k)`, `x.clear()`, `xs.insert(v, at: i)`

use crate::analyzer::*;
use crate::declaration_forms::lower_declarations;
use crate::lowering::nodes::{is_place, key};
use crate::node::{parens, prelude_call, Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{AT_WORD, DELETE_WORD, POP_WORD};

/// Methods that append one element: `x.add(v)` is `x += [v]`, in place
const APPEND_METHODS: [&str; 4] = ["add", "append", "push", "insert"];
/// Pseudo-call `list_drop_last(xs)`: xs without its last item, in place (wasm_emitter list_ops emit_list_drop_last)
pub const LIST_DROP_LAST: &str = "list_drop_last";
/// Pseudo-call `list_become(xs, ys)`: xs takes the items of the fresh list ys in place, so every holder of xs sees them
/// (`xs.pop(i)`, `xs.clear()`; wasm_emitter list_ops emit_list_become, card aliased-list)
pub const LIST_BECOME: &str = "list_become";
/// The list a pop template takes from, replaced by the variable or field popped
const POP_PLACE: &str = "pop_place";
pub const REMOVE_METHOD: &str = "remove";
/// Pseudo-call `removed_value(collection, k)`: what `collection.remove(k)` gives, by the collection's kind: of a map the
/// value of the key (P35, Python's dict.pop), of a list the list without the first element equal to k
pub const REMOVED_VALUE_CALL: &str = "removed_value";
/// lib/prelude.warp's `popped_item(x, k)` and `popped_rest(x, k)`: what Python's `xs.pop(i)` and `m.pop(k)` give and
/// leave, the element at the 0-based position i of a list, the value of the key k of a map (card python-aliases)
const POPPED_ITEM_CALL: &str = "popped_item";
const POPPED_REST_CALL: &str = "popped_rest";
/// Python's `xs.clear()` and `m.clear()`: the variable becomes lib/prelude.warp's `cleared(x)`, the empty list or map
const CLEAR_METHOD: &str = "clear";
const CLEARED_CALL: &str = "cleared";
pub(crate) const POP_TEMPORARY: &str = "pop_tmp";
pub(crate) const INSERT_METHOD: &str = "insert";

/// Pseudo-call `list_insert_at(list, position, value)`: the list with value inserted at the 0-based position
pub const INSERT_AT_CALL: &str = "list_insert_at";
/// Pseudo-call `insert_in_either_order(list, a, b)`: `xs.insert(a, b)` before the kinds decide which is the position
pub const INSERT_EITHER_CALL: &str = "insert_in_either_order";

/// The arguments of `xs.insert(…)`: `insert(x, at: i)` names the position, `insert(a, b)` leaves the order to the kinds
pub(crate) enum Inserted {
	At(Node, Node),
	EitherOrder(Node, Node),
}

/// `xs.add(v)`, `xs.append(v)`, `xs.push(a, b)`, `xs.insert(v)`: a call appending its arguments (`xs.insert(i, v)` inserts
/// at a position); the one table every pass asks
pub(crate) fn appends(method: &str, argument_count: usize) -> bool {
	APPEND_METHODS.contains(&method) && (method != INSERT_METHOD || argument_count == 1)
}

/// Methods that change the list variable they are called on: `xs.add(v)`, `xs.insert(v, at:1)`, `xs.pop()`
pub fn is_list_mutating_method(name: &str) -> bool {
	APPEND_METHODS.contains(&name) || [POP_WORD, REMOVE_METHOD, DELETE_WORD, CLEAR_METHOD].contains(&name)
}

fn is_removal_method(name: &str) -> bool {
	[REMOVE_METHOD, DELETE_WORD].contains(&name)
}

/// The element of `x.add(v)` when x is a variable or a field of one
pub(crate) fn appended_element<'a>(list: &Node, call: &'a Node) -> Option<&'a Node> {
	match called_on_place(list, call)? {
		(method, [element]) if APPEND_METHODS.contains(&method) => Some(element),
		_ => None,
	}
}

/// The method and the arguments of a call on a place
fn called_on_place<'a>(place: &Node, call: &'a Node) -> Option<(&'a str, &'a [Node])> {
	is_place(place).then(|| call.call_parts()).flatten()
}

/// `xs.pop()` when xs is a variable or a field of one (`s.items`): the last item, removed from xs in place, so every
/// holder of the list sees it (Python's list.pop(), P200b)
pub(crate) fn popped_list(list: &Node, call: &Node) -> Option<Node> {
	match called_on_place(list, call)? {
		(POP_WORD, []) => {
			let template = format!(
				"({POP_TEMPORARY} = {POP_PLACE}#count({POP_PLACE}); {POP_PLACE} = {LIST_DROP_LAST}({POP_PLACE}); {POP_TEMPORARY})"
			);
			Some(super::nodes::filled(&template, [(POP_PLACE, list.clone())]))
		}
		_ => None,
	}
}

/// `m.remove(k)` when m is a variable: the value of k, its entry removed from m (P35 default, Python's dict.pop); of a
/// list variable the first element equal to k removed, and the list it gives (REMOVED_VALUE_CALL). `delete` is the
/// familiar alias for `remove`; `x.pop(k)` is Python's pop of a key or a 0-based position (POPPED_ITEM_CALL).
pub(crate) fn removed_key(map: &Node, call: &Node) -> Option<Node> {
	let name = map.symbol_name()?;
	let (method, [map_key]) = call.call_parts()? else {
		return None;
	};
	let (value_word, rest_word) = match method {
		name if is_removal_method(name) => {
			if name == DELETE_WORD {
				crate::errors::diagnostic::note_alias(DELETE_WORD, REMOVE_METHOD);
			}
			(REMOVED_VALUE_CALL, crate::library_words::MAP_WITHOUT)
		}
		POP_WORD => (POPPED_ITEM_CALL, POPPED_REST_CALL),
		_ => return None,
	};
	let removed = Node::Symbol(format!("{name}{TEMPORARY_SEPARATOR}removed"));
	let value = prelude_call(value_word, vec![map.clone(), map_key.clone()]);
	let without = prelude_call(rest_word, vec![map.clone(), map_key.clone()]);
	let without = if rest_word == POPPED_REST_CALL { became(map, without) } else { without };
	Some(Node::List(
		vec![key(removed.clone(), Op::Assign, value), key(map.clone(), Op::Assign, without), removed],
		Bracket::Round,
		Separator::Semicolon,
	))
}

/// `x.clear()` when x is a variable: `x = cleared(x)`, emptied in place (CLEAR_METHOD)
pub(crate) fn cleared_collection(collection: &Node, call: &Node) -> Option<Node> {
	let _ = collection.symbol_name()?;
	matches!(call.call_parts()?, (CLEAR_METHOD, [])).then(|| {
		Node::Key(
			Box::new(collection.clone()),
			Op::Assign,
			Box::new(became(collection, prelude_call(CLEARED_CALL, vec![collection.clone()]))),
		)
	})
}

/// `list_become(place, fresh)`: the place takes the fresh list's items in place (LIST_BECOME)
fn became(place: &Node, fresh: Node) -> Node {
	parens(vec![Node::Symbol(LIST_BECOME.to_string()), place.clone(), fresh])
}

/// `xs.insert(a, b)` when xs is a variable. Warp writes `insert(value, position)`, Python `insert(position, value)`:
/// the order is never guessed (wiki/Footguns.md "Guessing intent"), `at:` or the kinds decide
pub(crate) fn inserted_element(list: &Node, call: &Node) -> Option<Inserted> {
	let (INSERT_METHOD, [first, second]) = called_on_place(list, call)? else {
		return None;
	};
	Some(match insert_arguments(call) {
		Some((value, position)) => Inserted::At(position.clone(), value.clone()),
		None => Inserted::EitherOrder(first.clone(), second.clone()),
	})
}

/// The value a and the position i of the call `insert(a, at: i)`, in either order
pub(crate) fn insert_arguments(call: &Node) -> Option<(&Node, &Node)> {
	match call.call_parts()? {
		(INSERT_METHOD, [value, at] | [at, value]) if insert_position(value).is_none() => {
			Some((value, insert_position(at)?))
		}
		_ => None,
	}
}

/// i of the argument `at: i`
pub(crate) fn insert_position(argument: &Node) -> Option<&Node> {
	match argument.drop_meta() {
		Node::Key(word, Op::Colon, position) if word.is_symbol(AT_WORD) => Some(position),
		_ => None,
	}
}

/// `xs = list_insert_at(xs, position, value)`, or `insert_in_either_order(xs, a, b)` that the emitter resolves by the
/// kinds of a and b: the one Int is the position, two Ints are ambiguous (Python and warp order differ)
pub(crate) fn lowered_insert(list: Box<Node>, call: &Node) -> Node {
	let (pseudo_call, first, second) = match inserted_element(&list, call).expect("guarded") {
		Inserted::At(position, value) => (INSERT_AT_CALL, position, value),
		Inserted::EitherOrder(first, second) => (INSERT_EITHER_CALL, first, second),
	};
	let arguments = vec![
		Node::Symbol(pseudo_call.to_string()),
		*list.clone(),
		lower_declarations(first),
		lower_declarations(second),
	];
	Node::Key(list, Op::Assign, Box::new(parens(arguments)))
}

/// `xs.add(v)` as `xs += [v]` and `xs.insert(i, v)` as its assignment, for a call that `updates_list`
pub(crate) fn list_update(list: Box<Node>, call: &Node) -> Node {
	match appended_element(&list, call).cloned() {
		Some(element) => {
			Node::Key(list, Op::AddAssign, Box::new(Node::List(vec![element], Bracket::Square, Separator::Space)))
		}
		None => lowered_insert(list, call),
	}
}

/// `xs.add(v)`, `xs.insert(i, v)`: an update of the list variable that ø (the empty list) allows
pub(crate) fn updates_list(list: &Node, call: &Node) -> bool {
	appended_element(list, call).is_some() || inserted_element(list, call).is_some()
}
