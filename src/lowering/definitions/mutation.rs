//! `fn!` mutates in place (user decision D2 2026-10-03, "by position", wiki/mutable.md): a `!` after a function or method
//! assigns its result back to the variable it was called on. `x.upper!`, `x.upper()!` and `upper(x)!` become
//! `x = x.upper` in the parser; in `upper x!` the parser only sees `x!`, so it marks x and `lower` turns the call into
//! `x = upper x`. A lone `x!`, `f!` or `{…}!` keeps evaluating, as does `x!` after an output word (`print friends!`).

use super::nodes::{assign, call, children_rewritten, int, written_entry};
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::words::{DELETE_WORD, POP_WORD, UNWRAP};

/// Meta key on a name written `x!`
pub(crate) const MUTATED_MARK: &str = "mutated";

/// The variable a call written before `!` changes: x in `x.upper`, `x.upper()` and `upper(x)`
pub fn mutated_variable(call: &Node) -> Option<&Node> {
	match call.drop_meta() {
		Node::Key(receiver, Op::Dot, method) if is_name(receiver) && is_method(method) => Some(receiver),
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[function, argument] if is_name(function) && is_name(argument) => Some(argument),
			_ => None,
		},
		_ => None,
	}
}

/// `x!`: evaluated alone, assigned the result of the call it is the argument of
pub fn marked(name: Node) -> Node {
	with_mark(name, Node::True)
}

/// `x!!`: like `x!`, and a block runs fully (blocks.rs)
pub fn marked_fully(name: Node) -> Node {
	with_mark(name, Node::int(FULLY))
}
const FULLY: i64 = 2;

/// The name marked with the mark's value: true for `x!`, FULLY for `x!!`
pub(crate) fn with_mark(name: Node, value: Node) -> Node {
	Node::Meta { node: Box::new(name), data: Box::new(Node::key(MUTATED_MARK, value)) }
}

/// The mark's value when a Meta layer's data is the mark of `x!`
pub(crate) fn mark_value(data: &Node) -> Option<&Node> {
	match data {
		Node::Key(key, _, value) if key.name() == MUTATED_MARK => Some(value),
		_ => None,
	}
}

/// What a `!` or `!!` follows (a name or a field `o.s1`), and whether it was `!!`
pub(crate) fn bang_target(node: &Node) -> Option<(Node, bool)> {
	let (target, value) = node.marked(MUTATED_MARK)?;
	Some((target.drop_meta().clone(), matches!(value.drop_meta(), Node::Number(n) if *n == FULLY)))
}

/// `delete xs[i]`, `delete xs#i`, `delete m["k"]`, `delete m.k` remove the entry as Python's `del`: `xs.pop(k)` of a
/// 0-based position or a key (card delete-list). A store's entry (`delete storage[k]`) was lowered by stored_values
pub fn lower_deletes(node: Node) -> Node {
	let node = node.map_children(lower_deletes);
	deleted_entry(&node).unwrap_or(node)
}

fn deleted_entry(statement: &Node) -> Option<Node> {
	let Node::List(items, _, _) = statement.drop_meta() else {
		return None;
	};
	let [word, target] = items.as_slice() else { return None };
	if !word.is_symbol(DELETE_WORD) {
		return None;
	}
	let (place, key, counted_from_one) = written_entry(target)?;
	let key = match key.drop_meta() {
		Node::Number(position) if counted_from_one => Node::Number(*position - Number::Int(1)),
		Node::Text(_) => key,
		_ if counted_from_one => super::nodes::key(key, Op::Sub, int(1)),
		_ => key,
	};
	Some(super::nodes::key(place.clone(), Op::Dot, call(POP_WORD, vec![key])))
}

/// `upper x!` → `x = upper x`; every other marked name is just the name
pub fn lower(node: Node) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(lower_inside).collect();
			let call = Node::List(items, bracket, separator);
			match &call {
				Node::List(items, _, separator)
					if items.len() == 2
						&& is_name(&items[0])
						&& !gives_nothing(&items[0])
						&& is_marked(&items[1])
						&& is_name(&unmarked(items[1].clone()))
						&& !matches!(separator, Separator::Semicolon | Separator::Newline) =>
				{
					let variable = unmarked(items[1].clone());
					assign(variable, strip_marks(call, false))
				}
				_ => strip_marks(call, true),
			}
		}
		// `x.upper!`: the parser marked the method it was parsing when it met the `!`
		Node::Key(receiver, Op::Dot, method) if is_name(&receiver) && is_marked(&method) => {
			let call = Node::Key(receiver.clone(), Op::Dot, Box::new(unmarked(*method)));
			Node::Key(receiver, Op::Assign, Box::new(call))
		}
		Node::Meta { .. } if is_marked(&node) => unwrapped(unmarked(node)),
		other => children_rewritten(other, lower),
	}
}

/// Items keep their marks until their own list decides; deeper lists are lowered on their own
fn lower_inside(item: Node) -> Node {
	if is_marked(&item) {
		item
	} else {
		lower(item)
	}
}

/// The marked names as plain names (the variable a mutation assigns) or, `unwrap`, as unwraps (`x!` alone)
fn strip_marks(node: Node, unwrap: bool) -> Node {
	let plain = |item: Node| {
		if unwrap {
			unwrapped(unmarked(item))
		} else {
			unmarked(item)
		}
	};
	match node {
		Node::List(items, bracket, separator) => Node::List(
			items.into_iter().map(|item| if is_marked(&item) { plain(item) } else { item }).collect(),
			bracket,
			separator,
		),
		other => other,
	}
}

/// `x!` that blocks.rs did not run: a name unwraps (row 27, `unwrap`), any other expression would be a block known only at
/// run time, which needs the run-time compiler (wiki/charged.md section 5): a loud error until it exists
fn unwrapped(name: Node) -> Node {
	// `x.upper!` (no block field, blocks.rs ran those): x = x.upper (D2)
	if let Node::Key(receiver, Op::Dot, method) = name.drop_meta() {
		if is_name(receiver) && is_method(method) {
			return Node::Key(receiver.clone(), Op::Assign, Box::new(name.clone()));
		}
	}
	// `ø!`: P73 force — Empty unwraps like a name holding ø
	if name.is_empty_node() {
		return call(UNWRAP, vec![name]);
	}
	// `now!`: the time the run reads where it stands (card data-run-now)
	if crate::time::is_time_call(&name) {
		return name;
	}
	if !is_name(&name) {
		let written = crate::normalize::operand_text(&name);
		return crate::node::error(&format!("{written} is only known at run time: `!` needs a constant block"));
	}
	call(UNWRAP, vec![name])
}

/// The mark may sit under other meta information, such as the position the parser records
pub(crate) fn is_marked(node: &Node) -> bool {
	match node {
		Node::Meta { node, data } => is_mark(data) || is_marked(node),
		_ => false,
	}
}

fn is_mark(data: &Node) -> bool {
	mark_value(data).is_some()
}

fn unmarked(node: Node) -> Node {
	match node {
		Node::Meta { node, data } if is_mark(&data) => *node,
		meta @ Node::Meta { .. } => meta.under_meta(unmarked),
		other => other,
	}
}

/// An output word: its result is nothing, so `print friends!` prints the forced friends and leaves them (decision
/// print-bang); assigning its ø back would empty the variable
fn gives_nothing(word: &Node) -> bool {
	matches!(word.drop_meta(), Node::Symbol(name) if crate::wasm_emitter::OUTPUT_CALLS.contains(&name.as_str()))
}

fn is_name(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(_))
}

fn is_method(method: &Node) -> bool {
	match method.drop_meta() {
		Node::Symbol(_) => true,
		Node::List(items, _, _) => items.first().is_some_and(is_name),
		_ => false,
	}
}
