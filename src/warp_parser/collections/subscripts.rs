//! Subscripts: `xs[i]` as the 1-based `xs#(i+1)`, negative indexes counting from the end, slices `s[2…]` and `xs#(a..b)`
use super::*;

/// The marks of a slice from the start, `s[…3]`: the inclusive ones before `..`, which they begin with
const SLICE_FROM_START: [(&str, Op); 4] = [(":", Op::Colon), ("…", Op::To), ("...", Op::To), ("..", Op::Range)];

/// The name before a minus written apart, `f - 2`: the subtraction, where `f -2` applies a callable f to -2, as the
/// math words do (`abs - 2`, `abs -2`; negated_calls::lower_negated_calls)
#[derive(Clone, Debug, PartialEq)]
pub struct SpacedMinus;

/// The number a subscript was written with, `1` of `d[1]`, kept on its shifted index `d#2`: a map keyed by numbers
/// reads it (number_keys.rs, P34)
#[derive(Clone, Debug, PartialEq)]
pub struct WrittenIndex(pub Number);

/// The `1` a subscript adds to its index, `xs[i]` read `xs#(i+1)`: unlike a `+ 1` written by hand, a position meeting
/// the brackets drops it (index_positions.rs, card index-position)
#[derive(Clone, Debug, PartialEq)]
pub struct SubscriptOffset;

/// `target[index]` is the 1-based `target#(index+1)`; a numeric index is shifted at parse time.
/// A slice `target[start:end]`, `target[start..end]`, `target[start...last]` is the call `slice(target, start, end)`.
/// A negative index is an error that names the way to the last element: warp never wraps around (Footguns.md).
pub fn subscript(target: Node, index: Node) -> Node {
	let positions = match index.drop_meta() {
		Node::Key(start, _, end) if slice_bounds(&index).is_some() => vec![start.as_ref(), end.as_ref()],
		_ => vec![&index],
	};
	if let Some(negative) = positions.into_iter().find(|position| counts_from_the_end(position)) {
		let message = format!(
			"index out of range: negative index {} does not wrap around, the last element is last({})",
			negative.serialize(),
			target.serialize()
		);
		return Diagnostic::at(negative, message).into_error();
	}
	if let Some((start, end)) = slice_bounds(&index) {
		return call(SLICE_WORD, vec![target, start, end]);
	}
	let one = Node::meta(Node::Number(Number::Int(1)), Node::data(SubscriptOffset));
	let one_based = match index.drop_meta() {
		Node::Number(n) => Node::meta(Node::Number(*n + Number::Int(1)), Node::data(WrittenIndex(*n))),
		_ => Node::Key(Box::new(index), Op::Add, Box::new(one)),
	};
	Node::Key(Box::new(target), Op::Hash, Box::new(one_based))
}

/// A negative number or a negated expression: `-1`, `-k`
pub(crate) fn counts_from_the_end(index: &Node) -> bool {
	match index.drop_meta() {
		Node::Number(n) => f64::from(*n) < 0.0,
		Node::Key(empty, Op::Sub, _) => empty.is_empty_node(),
		_ => false,
	}
}

/// The 0-based start and exclusive end of a slice index, ø where omitted (Python's `a[1:]`, `a[:2]`);
/// an inclusive range `start...last` ends after last
pub(crate) fn slice_bounds(index: &Node) -> Option<(Node, Node)> {
	let Node::Key(start, op, end) = index.drop_meta() else {
		return None;
	};
	let end = match op {
		Op::Colon | Op::Range => end.as_ref().clone(),
		Op::To => {
			unless_open(end, |end| Node::Key(Box::new(end.clone()), Op::Add, Box::new(Node::Number(Number::Int(1)))))
		}
		_ => return None,
	};
	Some((start.as_ref().clone(), end))
}

/// `xs#a..b` reads as the range from the value xs#a; P58 (user: "create a strong warning"): a warning naming the slice
/// `xs#(a..b)` and the range `(xs#a)..b`
pub(crate) fn hash_range_warning(lhs: &Node, op: Op, written: &str, rhs: &Node) -> Option<Diagnostic> {
	let Node::Key(target, Op::Hash, index) = lhs.drop_meta() else {
		return None;
	};
	if !matches!(op, Op::Range | Op::To) {
		return None;
	}
	let (target, index, end) = (target.serialize(), index.serialize(), rhs.serialize());
	let (target, index, end) = (target.trim(), index.trim(), end.trim());
	let (as_written, slice, range) = (
		format!("{target}#{index}{written}{end}"),
		format!("{target}#({index}{written}{end})"),
		format!("({target}#{index}){written}{end}"),
	);
	let message = format!("{as_written} is the range from the value {target}#{index}: for the slice write {slice}, for the range write {range}");
	Some(Diagnostic::at(lhs, message).offer("the slice", &as_written, slice).offer(
		"the range from the value",
		&as_written,
		range,
	))
}

/// `xs#(a…b)`, `xs#(a..b)`: the 0-based start and exclusive end of a 1-based slice (`xs#a` is 1-based); only a range in
/// parentheses, `xs#a..b` stays the range from the value xs#a
pub(crate) fn hash_slice_bounds(index: &Node) -> Option<(Node, Node)> {
	let Node::List(items, Bracket::Round, _) = index.drop_meta() else {
		return None;
	};
	let [range] = items.as_slice() else { return None };
	let Node::Key(start, op @ (Op::Range | Op::To), end) = range.drop_meta() else {
		return None;
	};
	let minus_one = |bound: &Node| Node::Key(Box::new(bound.clone()), Op::Sub, Box::new(Node::Number(Number::Int(1))));
	let end = if *op == Op::To { end.as_ref().clone() } else { unless_open(end, minus_one) };
	Some((unless_open(start, minus_one), end))
}

/// A slice bound shifted by `shift`; an open bound (`s[2…]`, `s[…3]`) stays open, ø
pub(crate) fn unless_open(bound: &Node, shift: impl Fn(&Node) -> Node) -> Node {
	match bound.drop_meta() {
		Node::Empty => Node::Empty,
		_ => shift(bound),
	}
}

/// The written index of a subscript's 1-based index `index+1`, when it was not a number (inverse of `subscript`)
pub fn subscript_key(one_based_index: &Node) -> Option<&Node> {
	match one_based_index.drop_meta() {
		Node::Key(key, Op::Add, one)
			if matches!(one.drop_meta(), Node::Number(crate::extensions::numbers::Number::Int(1))) =>
		{
			Some(key)
		}
		_ => None,
	}
}

pub(crate) fn is_unindexable_keyword(node: &Node) -> bool {
	node.symbol_name().is_some_and(|word| UNINDEXABLE_KEYWORDS.contains(&word))
}

/// `id=1 kind="x"` in brackets: each attribute
pub(crate) fn attribute_items(index: &Node) -> Vec<Node> {
	match index.drop_meta() {
		Node::List(items, Bracket::None, _) => items.clone(),
		single => vec![single.clone()],
	}
}

impl WarpParser {
	/// A loop's key variable (`for k in keys(m)`) or a name holding a text: `m[k]` looks a key up, no indexing hint
	pub(crate) fn names_a_key(&self, index: &Node) -> bool {
		matches!(index.drop_meta(), Node::Symbol(name) if self.key_variables.contains(name) || self.text_variables.contains(name))
	}

	pub(crate) fn try_parse_subscript(&mut self, lhs: &Node, min_bp: u8, subscript_bp: u8) -> Option<Node> {
		// a space before the bracket makes it a list, `f [1, 2]`; only `xs[1]` indexes
		if self.current_char() != '['
			|| min_bp > subscript_bp
			|| self.prev_char().is_whitespace()
			|| is_unindexable_keyword(lhs)
		{
			return None;
		}
		if let Some(list_type) = self.try_parse_array_type_suffix(lhs) {
			return Some(list_type);
		}
		let before_bracket = self.mark();
		self.advance(); // skip '['
		self.skip_whitespace();

		let outer_spaced_list = std::mem::replace(&mut self.in_spaced_list, false); // `xs[i -1]` subtracts
		let mut indices = vec![self.parse_slice_from_start().unwrap_or_else(|| self.parse_expr(0))];
		self.skip_whitespace();

		while self.current_char() == ',' {
			self.advance(); // skip ','
			self.skip_whitespace();
			indices.push(self.parse_expr(0));
			self.skip_whitespace();
		}
		self.in_spaced_list = outer_spaced_list;

		if self.current_char() != ']' {
			self.rewind(before_bracket); // `foo [1 2 3]`: a list argument, not an index
			return None;
		}
		self.advance(); // skip ']'
		if let Some(node) = self.try_attributed_body(lhs, &indices) {
			return Some(node);
		}
		// `int[n]` is n zeros of the type unless int is a variable (analyzer lower_declarations): no indexing hint
		let names_a_type = lhs.symbol_name().is_some_and(|word| crate::filled_lists::zero_list(Empty, word).is_some());
		if slice_bounds(&indices[0]).is_none() && !names_a_type && !self.names_a_key(&indices[0]) {
			crate::normalize::set_position_of(lhs);
			norm::index_operator(
				&crate::normalize::operand_text(lhs),
				&crate::normalize::operand_text(&indices[0]),
				true,
			);
		}

		Some(indices.into_iter().fold(lhs.clone(), subscript))
	}

	/// `a[id=1]{…}` (wiki/reference.md): attributes in brackets before a body are its meta entries, `a{@id:1 …}`
	fn try_attributed_body(&mut self, lhs: &Node, indices: &[Node]) -> Option<Node> {
		let attributes: Vec<Node> = indices
			.iter()
			.flat_map(attribute_items)
			.map(|item| match item.drop_meta() {
				Node::Key(key, Op::Assign | Op::Colon, value) if matches!(key.drop_meta(), Symbol(_)) => Some(
					Node::Key(Box::new(Symbol(format!("{ATTRIBUTE_MARK}{}", key.name()))), Op::Colon, value.clone()),
				),
				_ => None,
			})
			.collect::<Option<_>>()?;
		let mut offset = 0;
		while self.peek_char(offset) == ' ' {
			offset += 1;
		}
		if !matches!(lhs.drop_meta(), Symbol(_)) || self.peek_char(offset) != '{' {
			return None;
		}
		self.skip_spaces();
		let Node::List(items, bracket, separator) = self.parse_bracketed('{') else {
			return None;
		};
		let body = Node::List([attributes, items].concat(), bracket, separator);
		Some(Node::Key(Box::new(lhs.clone()), Op::Colon, Box::new(body)))
	}

	/// `[:end]`, `[…last]`, `[..end]` and `[:]`, a slice from the start: `ø:end`
	pub(crate) fn parse_slice_from_start(&mut self) -> Option<Node> {
		let (op, chars) = SLICE_FROM_START
			.iter()
			.copied()
			.find(|(written, _)| written.chars().enumerate().all(|(offset, ch)| self.peek_char(offset) == ch))
			.map(|(written, op)| (op, written.chars().count()))?;
		self.advance_by(chars);
		self.skip_whitespace();
		let end = if self.current_char() == ']' { Empty } else { self.parse_expr(0) };
		Some(Node::Key(Box::new(Empty), op, Box::new(end)))
	}
}
