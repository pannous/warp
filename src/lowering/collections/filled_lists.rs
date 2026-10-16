//! Filled lists: `int[100]`, `x:int[100]`, `n times [x]`, and the ambiguous `[x]*n` and `[1 2 3]+4`

use crate::analyzer::*;
use crate::broadcasting::{element_wise, EACH_ELEMENT};
use crate::extensions::numbers::Number;
use crate::lowering::nodes::{call, key};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_kinds::Kind;
use crate::words::FLOAT_WORD;
use std::collections::HashSet;

/// Pseudo-call `zero_fill(count, zero)`: the emitter builds the zero-filled list of `count` elements with a runtime loop,
/// so neither the program nor the compiler grows with the size of the array
pub const ZERO_FILL_CALL: &str = "zero_fill";

pub(crate) const LIST_TIMES_TOPIC: &str = "list-times";

pub(crate) const LIST_PLUS_TOPIC: &str = "list-plus";

/// The zero value of an element type word (`int`, `ints`, `float`, `text` …)
pub(crate) fn zero_element(type_word: &str) -> Option<Node> {
	let element = plural_element_type(type_word).unwrap_or(type_word);
	Some(match type_word_kind(element)? {
		Kind::Int => Node::int(0),
		// `0.0f`: a bare 0.0 is an exact decimal, so an Int
		Kind::Float => key(Node::float(0.0), Op::As, Node::Symbol(FLOAT_WORD.to_string())),
		Kind::Text => Node::Text(String::new()),
		Kind::Codepoint => Node::Char('\0'),
		_ => return None,
	})
}

/// The zero-filled list of `count` (any number expression) elements of the type word
pub fn zero_list(count: Node, type_word: &str) -> Option<Node> {
	let zero = zero_element(type_word)?;
	Some(call(ZERO_FILL_CALL, vec![count, zero]))
}

/// `[x]*n` and `n*[x]` with a list literal: Python repeats the list, NumPy multiplies each element (wiki/Footguns.md
/// "Lists and arithmetic"), so the user is asked; unanswered it is an error naming both explicit forms.
/// Likewise `[1 2 3]+4`: append or add to each element?
pub fn lower_list_times(node: Node) -> Node {
	let list_arithmetic =
		|key: &Node, positioned: &Node| list_times(key, positioned).or_else(|| list_plus(key, positioned));
	match node {
		Node::Meta { node: inner, data } => {
			let positioned = Node::Meta { node: inner.clone(), data: data.clone() };
			list_arithmetic(&inner, &positioned)
				.unwrap_or_else(|| Node::Meta { node: Box::new(lower_list_times(*inner)), data })
		}
		Node::Key(left, op, right) => {
			let key = key(lower_list_times(*left), op, lower_list_times(*right));
			list_arithmetic(&key, &key).unwrap_or(key)
		}
		Node::List(items, bracket, separator) => {
			Node::List(items.into_iter().map(lower_list_times).collect(), bracket, separator)
		}
		other => other,
	}
}

pub(crate) fn list_times(key: &Node, positioned: &Node) -> Option<Node> {
	let Node::Key(left, Op::Mul, right) = key.drop_meta() else {
		return None;
	};
	// `[2 2] .* [2 4]`: the dotted operator said element-wise, broadcasting pairs the two lists
	let is_each_element = |side: &Node| side.is_symbol(EACH_ELEMENT);
	if is_each_element(left) || is_each_element(right) {
		return None;
	}
	let (list, count) = match (is_list_literal(left), is_list_literal(right)) {
		(true, false) => (left.as_ref(), right.as_ref()),
		(false, true) => (right.as_ref(), left.as_ref()),
		_ => return None,
	};
	let written = key.drop_meta().serialize();
	let (list_text, count_text) = (crate::normalize::operand_text(list), crate::normalize::operand_text(count));
	let readings = vec![
		crate::errors::diagnostic::reading("repeat the list", &format!("{count_text} times {list_text}")),
		crate::errors::diagnostic::reading("multiply each element", &format!("{list_text}.map(x => x*{count_text})")),
	];
	let question = crate::errors::diagnostic::Ask::new(
		LIST_TIMES_TOPIC,
		format!("type error: list * number: does `{written}` repeat the list or multiply each element?"),
		readings,
		crate::errors::diagnostic::Fallback::Error,
	)
	.written(&written)
	.at_node(positioned);
	Some(match crate::errors::diagnostic::ask(&question) {
		Ok(0) => filled_list(count.clone(), list)
			.unwrap_or_else(|| crate::node::error("`n times [x]` repeats one element: `3 times [0]`")),
		Ok(_) => lower_list_times(element_wise(list.clone(), Op::Mul, count.clone())),
		Err(error) => error,
	})
}

pub(crate) fn is_list_literal(side: &Node) -> bool {
	matches!(side.drop_meta(), Node::List(_, Bracket::Square, _))
}

/// `[1 2 3]+4` and `4+[1 2 3]` with a list literal and a number: append (prepend) or add to each element?
/// Unanswered it is an error naming `[1 2 3] + [4]` and `[1 2 3] .+ 4`
pub(crate) fn list_plus(sum: &Node, positioned: &Node) -> Option<Node> {
	let Node::Key(left, Op::Add, right) = sum.drop_meta() else {
		return None;
	};
	let is_number = |side: &Node| matches!(side.drop_meta(), Node::Number(_));
	let (list, number, list_first) = match (is_list_literal(left), is_list_literal(right)) {
		(true, false) if is_number(right) => (left.as_ref(), right.as_ref(), true),
		(false, true) if is_number(left) => (right.as_ref(), left.as_ref(), false),
		_ => return None,
	};
	let written = sum.drop_meta().serialize();
	let (list_text, number_text) = (crate::normalize::operand_text(list), crate::normalize::operand_text(number));
	let (joining, joined) = match list_first {
		true => ("append", format!("{list_text} + [{number_text}]")),
		false => ("prepend", format!("[{number_text}] + {list_text}")),
	};
	let readings = vec![
		crate::errors::diagnostic::reading(joining, &joined),
		crate::errors::diagnostic::reading("add to each element", &format!("{list_text} .+ {number_text}")),
	];
	let question = crate::errors::diagnostic::Ask::new(
		LIST_PLUS_TOPIC,
		format!("type error: list + number: does `{written}` {joining} {number_text} or add it to each element?"),
		readings,
		crate::errors::diagnostic::Fallback::Error,
	)
	.written(&written)
	.at_node(positioned);
	let singleton = Node::List(vec![number.clone()], Bracket::Square, Separator::Space);
	Some(match crate::errors::diagnostic::ask(&question) {
		Ok(0) if list_first => key(list.clone(), Op::Add, singleton),
		Ok(0) => key(singleton, Op::Add, list.clone()),
		Ok(_) => element_wise(list.clone(), Op::Add, number.clone()),
		Err(error) => error,
	})
}

/// `n times [x]`: the list of n copies of x (`zero_fill(n, x)`); `[x]*n` stays ambiguous (Python repeats, NumPy multiplies)
pub fn filled_list(count: Node, list: &Node) -> Option<Node> {
	let Node::List(items, Bracket::Square, _) = list.drop_meta() else {
		return None;
	};
	let [element] = items.as_slice() else { return None };
	Some(call(ZERO_FILL_CALL, vec![count, element.clone()]))
}

/// `int[n]` parsed as the subscript `int#(n+1)`: the list of n zeros when the type word is no variable
pub(crate) fn zero_filled_subscript(element: &Node, one_based: &Node, variables: &HashSet<String>) -> Option<Node> {
	let word = element.symbol_name()?;
	if variables.contains(word) {
		return None;
	}
	let count = match one_based.drop_meta() {
		Node::Number(Number::Int(one_based)) => Node::int(one_based - 1),
		other => crate::warp_parser::subscript_key(other)?.clone(),
	};
	zero_list(count, word)
}

/// `int[100]` or `100 * int`: the zero-filled list of that many elements (the parser reads `int[n]` as one already),
/// unless the subscripted word is one of the variables (`chars=["a" "b"]; x=chars[1]` indexes)
pub(crate) fn typed_array_value(value: &Node, variables: &HashSet<String>) -> Option<Node> {
	match value.drop_meta() {
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(call)) if call == ZERO_FILL_CALL) => {
			Some(value.clone())
		}
		Node::Key(count, Op::Mul, element) => match element.drop_meta() {
			Node::Symbol(word) => zero_list(count.as_ref().clone(), word),
			_ => None,
		},
		Node::Key(element, Op::Hash, one_based) => zero_filled_subscript(element, one_based, variables),
		_ => None,
	}
}

/// `x:int[100]` as the variable and its zero-filled list
pub(crate) fn typed_array_declaration(node: &Node, variables: &HashSet<String>) -> Option<(Node, Node)> {
	let Node::Key(name, Op::Colon, type_node) = node.drop_meta() else {
		return None;
	};
	let _ = name.symbol_name()?;
	Some((name.drop_meta().clone(), typed_array_value(type_node, variables)?))
}

/// `x : 100` followed by the element type `int`: the declaration `x = [0 … 0]`
pub(crate) fn counted_array_declaration(declared_count: &Node, type_word: &Node) -> Option<Node> {
	let Node::Key(name, Op::Colon, count) = declared_count.drop_meta() else {
		return None;
	};
	match (name.drop_meta(), count.drop_meta(), type_word.drop_meta()) {
		(Node::Symbol(_), Node::Number(Number::Int(count)), Node::Symbol(word)) => Some(Node::Key(
			Box::new(name.drop_meta().clone()),
			Op::Assign,
			Box::new(zero_list(Node::int(*count), word)?),
		)),
		_ => None,
	}
}
