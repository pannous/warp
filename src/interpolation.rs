//! Interpolated text (user decision D1, 2026-10-03): double-quoted text in code takes Swift holes `"a \(x+1) b"` and
//! dollar holes `"${x+1}"`; `\(…)` is the canonical form the normalizer hints. A bare `"$x"` is text ("only the one with
//! the curly braces must interpolate the other is text like dollar money"), single quotes stay literal.
//!
//! The parser keeps such a literal as its template text (holes written `${expr}`, a literal dollar `$$`, the syntax of
//! injection::parts) marked as a template, so `sql "…"` / `sh "…"` still see the literal and turn holes into parameters.
//! Every other template becomes text building here: `"a \(x) b"` → `"a " + text_form(x) + " b"`.

use crate::injection::{parts_with, Part};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasm_emitter::text_builtins::TEXT_FORM;

/// Meta key marking a text literal as a template
const TEMPLATE_MARK: &str = "template";

/// The text literal of a template, holes in `${expr}` form and literal dollars as `$$`
pub fn template_text(template: &str) -> Node {
	Node::meta(Node::Text(template.to_string()), Node::key(TEMPLATE_MARK, Node::True))
}

fn is_template_mark(data: &Node) -> bool {
	matches!(data, Node::Key(name, _, _) if matches!(name.as_ref(), Node::Symbol(mark) | Node::Text(mark) if mark == TEMPLATE_MARK))
}

/// Does a hole of a template mention a name (`"caught: \(e)"` mentions e), as `mentions` tells of each hole
pub fn template_mentions(node: &Node, mentions: impl Fn(&Node) -> bool) -> bool {
	let Node::Meta { node, data } = node else { return false };
	if !is_template_mark(data) {
		return template_mentions(node, mentions);
	}
	let Node::Text(template) = node.drop_meta() else { return false };
	parts_with(template, false).is_ok_and(|parts| parts.iter().any(|part| matches!(part, Part::Hole(expression) if mentions(expression))))
}

/// The expressions in the holes of a template (`"a \(x.round(2))"` → x.round(2)); none for other nodes
pub fn holes(node: &Node) -> Vec<Node> {
	let found = std::cell::RefCell::new(vec![]);
	template_mentions(node, |hole| {
		found.borrow_mut().push(hole.clone());
		false
	});
	found.into_inner()
}

/// The template as the concatenation of its pieces now, when a hole mentions a name (`"\(Math.max(1, 5))"` mentions
/// Math): for a pass that gives such names their meaning before this lowering runs
pub fn interpolated_mentioning(node: &Node, mentions: impl Fn(&Node) -> bool) -> Option<Node> {
	let Node::Meta { node: inner, data } = node else { return None };
	match inner.drop_meta() {
		Node::Text(template) if is_template_mark(data) && template_mentions(node, mentions) => Some(interpolated(template)),
		_ => None,
	}
}

/// The first source pass: `sql "…"` / `sh "…"` templates become parameterized calls, every other template the
/// concatenation of its pieces, so each later pass reads the holes as code (`"\(area.params)"`)
pub fn lower_program(program: Node) -> Node {
	crate::injection::lower_templates(program).map(lower).unwrap_or_else(|error| error)
}

/// Every template left after the sql/sh lowering becomes the concatenation of its pieces
pub fn lower(node: Node) -> Node {
	match node {
		Node::Meta { node, data } if is_template_mark(&data) => match node.drop_meta() {
			Node::Text(template) => interpolated(template),
			_ => *node,
		},
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(lower).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(lower(*left)), op, Box::new(lower(*right))),
		Node::Type { name, body } => Node::Type { name, body: Box::new(lower(*body)) },
		Node::Error(inner) => Node::Error(Box::new(lower(*inner))),
		other => other,
	}
}

fn interpolated(template: &str) -> Node {
	let pieces = match parts_with(template, false) {
		Ok(pieces) => pieces,
		Err(error) => return crate::node::error(&error.0),
	};
	let mut pieces = pieces.into_iter().map(|part| match part {
		Part::Literal(text) => Node::Text(text),
		Part::Hole(expression) => Node::List(vec![Node::Symbol(TEXT_FORM.into()), lower(expression)], Bracket::Round, Separator::None),
	});
	let first = pieces.next().unwrap_or(Node::Text(String::new()));
	pieces.fold(first, |text, piece| Node::Key(Box::new(text), Op::Add, Box::new(piece)))
}
