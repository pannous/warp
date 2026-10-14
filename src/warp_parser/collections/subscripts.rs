//! Subscripts: `xs[i]` as the 1-based `xs#(i+1)`, negative indexes counting from the end, slices `s[2…]` and `xs#(a..b)`
use super::*;

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
		Node::Key(empty, Op::Sub, _) => matches!(empty.drop_meta(), Empty),
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
		Op::To => unless_open(end, |end| {
			Node::Key(Box::new(end.clone()), Op::Add, Box::new(Node::Number(Number::Int(1))))
		}),
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
	Some(
		Diagnostic::at(lhs, message)
			.offer("the slice", &as_written, slice)
			.offer("the range from the value", &as_written, range),
	)
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
	let end = if *op == Op::To {
		end.as_ref().clone()
	} else {
		unless_open(end, minus_one)
	};
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
			if matches!(
				one.drop_meta(),
				Node::Number(crate::extensions::numbers::Number::Int(1))
			) =>
		{
			Some(key)
		}
		_ => None,
	}
}
