//! A node as plain warp data, the view the warp-written compiler parts in lib/compiler/ read and give (walk.warp
//! node_of / view_of, notes/self_hosting.md): a number, bool or ø is itself, every other node a list tagged by its kind.
//! The twin tests (tests/common/twins.rs) and the self-hosted pipeline (self_hosted.rs) both cross this bridge.
use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};

/// A list as warp writes it: the empty list is ø
pub fn warp_list(items: impl Iterator<Item = Node>) -> Node {
	let items: Vec<Node> = items.collect();
	if items.is_empty() {
		return Node::Empty;
	}
	Node::List(items, Bracket::Square, Separator::None)
}

pub fn node_view(node: &Node) -> Node {
	Viewer::default().view(node)
}

/// The view with the source position of each node that has one, ["at", line, column, view] (card carries-line): a pass
/// twin reports where its Rust twin does
pub fn positioned_view(node: &Node) -> Node {
	Viewer { positions: true.into(), ..Viewer::default() }.view(node)
}

/// The plain view with each data node and each number but an int or float (a time, `2π`: no view tells it whole) as
/// ["data", "#i"] or ["number", "#i"], its handle in
/// the returned nodes: `node_of_handled_view` puts the node back where a warp stage left the handle
pub fn handled_plain_view(node: &Node) -> (Node, Vec<Node>) {
	let viewer = Viewer { handles: Some(std::cell::RefCell::new(vec![])), marks: true, ..Viewer::default() };
	// the attribute layers stay: the Rust passes after the warp stage read them (the mark of `x!`, `extends`)
	let view = without_layers(viewer.view(node), &["literal"]);
	(view, viewer.handles.map(std::cell::RefCell::into_inner).unwrap_or_default())
}

#[derive(Default)]
struct Viewer {
	positions: std::cell::Cell<bool>,
	handles: Option<std::cell::RefCell<Vec<Node>>>,
	/// the mark of `x!` as an attribute layer: the bridge hands it on, a source's own view has no such annotation
	marks: bool,
}

impl Viewer {
	fn view(&self, node: &Node) -> Node {
		match node {
			Node::True | Node::False | Node::Empty => node.clone(),
			Node::Number(number @ (Number::Int(_) | Number::Float(_))) => Node::Number(*number),
			Node::Number(number) => {
				tagged("number", [self.handle(node).unwrap_or_else(|| Node::text(&number.to_string()))])
			}
			Node::Char(c) => tagged("char", [Node::Char(*c)]),
			Node::Text(text) => tagged("text", [Node::text(text)]),
			Node::Symbol(name) => tagged("symbol", [Node::text(name)]),
			// an error and a type stay opaque to a pass twin (walk.warp node_of): no positions inside
			Node::Error(inner) => tagged("error", [self.without_positions(|viewer| viewer.view(inner))]),
			Node::Key(left, op, right) => tagged("key", [Node::text(op.as_str()), self.view(left), self.view(right)]),
			Node::List(items, bracket, separator) => {
				let mut parts = vec![Node::text(&bracket_glyphs(bracket)), Node::text(&separator_glyph(separator))];
				parts.extend(items.iter().map(|item| self.view(item)));
				tagged("list", parts)
			}
			Node::Type { name, body } => {
				tagged("type", self.without_positions(|viewer| [viewer.view(name), viewer.view(body)]))
			}
			Node::Data(data) => tagged(
				"data",
				[self.handle(node).unwrap_or_else(|| {
					Node::text(&crate::numbers::units::describe(data).unwrap_or_else(|| format!("Data({data:?})")))
				})],
			),
			Node::Meta { node: inner, .. } => match node.get_lineinfo() {
				Some(info) if self.positions.get() && inner.get_lineinfo().is_none() => {
					let at = [Node::int(info.line_nr as i64), Node::int(info.column as i64)];
					tagged("at", at.into_iter().chain([self.meta_view(node, inner)]))
				}
				_ => self.meta_view(node, inner),
			},
		}
	}

	/// The handle of a node no view tells whole (a time, `2π`) when handles are kept
	fn handle(&self, node: &Node) -> Option<Node> {
		let mut handles = self.handles.as_ref()?.borrow_mut();
		handles.push(node.clone());
		Some(Node::text(&format!("{HANDLE_MARK}{}", handles.len() - 1)))
	}

	/// What `view` gives without positions, the same handles kept
	fn without_positions<T>(&self, view: impl FnOnce(&Viewer) -> T) -> T {
		let positions = self.positions.replace(false);
		let viewed = view(self);
		self.positions.set(positions);
		viewed
	}

	/// A source literal (`01234`, `1.10`) and an annotation (`@name(value)`) survive, comments go
	fn meta_view(&self, node: &Node, inner: &Node) -> Node {
		if let Some(template) = crate::text::interpolation::template_of(node) {
			return tagged("template", [Node::text(template)]);
		}
		if let Some(literal) = node.source_literal() {
			return tagged("literal", [Node::text(literal), self.view(inner)]);
		}
		let attributes = node.attributes();
		if attributes.len() > inner.attributes().len() {
			let (name, value) = attributes[0];
			return tagged("attribute", [Node::text(name), self.view(value), self.view(inner)]);
		}
		let mark = match node {
			Node::Meta { data, .. } if self.marks => bridged_mark(data),
			_ => None,
		};
		if let Some((name, value)) = mark {
			return tagged("attribute", [Node::text(name), self.view(value), self.view(inner)]);
		}
		self.view(inner)
	}
}

/// The marks the bridge hands on as attribute layers, which the Rust passes after a warp stage read: `x!`'s,
/// `function f`'s (closures.rs function_reference) and a function body's doc and place (doc_strings.rs), which
/// reflection.rs and doc_strings.rs make before number_words' twin runs
const BRIDGED_MARKS: [&str; 4] = [
	crate::mutation::MUTATED_MARK,
	crate::closures::FUNCTION_REFERENCE_MARK,
	crate::lowering::doc_strings::DOC_KEY,
	crate::lowering::doc_strings::WRITTEN_AT_KEY,
];

/// The bridged mark a Meta layer's data is, with its value
fn bridged_mark(data: &Node) -> Option<(&'static str, &Node)> {
	match data {
		Node::Key(key, _, value) => {
			BRIDGED_MARKS.into_iter().find(|mark| key.name() == *mark).map(|mark| (mark, value.as_ref()))
		}
		_ => None,
	}
}

/// What a data handle's text starts with
const HANDLE_MARK: &str = "#";

fn tagged(kind: &str, parts: impl IntoIterator<Item = Node>) -> Node {
	warp_list(std::iter::once(Node::text(kind)).chain(parts))
}

/// The opening glyph ("{", "[", "(", "<", "" for none), both glyphs for another pair
fn bracket_glyphs(bracket: &Bracket) -> String {
	match bracket {
		Bracket::None => String::new(),
		Bracket::Other(open, close) => format!("{open}{close}"),
		_ => bracket.opening().to_string(),
	}
}

fn bracket_of_glyphs(glyphs: &str) -> Option<Bracket> {
	let mut chars = glyphs.chars();
	Some(match (chars.next(), chars.next()) {
		(None, _) => Bracket::None,
		(Some('{'), None) => Bracket::Curly,
		(Some('['), None) => Bracket::Square,
		(Some('('), None) => Bracket::Round,
		(Some('<'), None) => Bracket::Less,
		(Some(open), Some(close)) => Bracket::Other(open, close),
		_ => return None,
	})
}

fn separator_glyph(separator: &Separator) -> String {
	separator.to_char().map(String::from).unwrap_or_default()
}

fn separator_of_glyph(glyph: &str) -> Option<Separator> {
	Some(match glyph {
		"" => Separator::None,
		" " => Separator::Space,
		"," => Separator::Comma,
		";" => Separator::Semicolon,
		"\n" => Separator::Newline,
		"\t" => Separator::Tab,
		_ => return None,
	})
}

/// The view as warp gives it back: no literal and attribute layers, a character as a text (warp holds a one-character
/// text as the character, card character-text), an empty group as ø (warp's empty `{}` / `()` is ø, decision
/// empty-group-node)
pub fn plain_view(node: &Node) -> Node {
	plain(node_view(node))
}

/// The plain view the analysis reads: each declared type shown in it
pub fn analysis_view(program: &Node) -> Node {
	plain_view(&declared_types_shown(program.clone()))
}

/// `float64 r = 3.14159` keeps its type as metadata on the target, which a view drops: shown as `r: float64 = 3.14159`
fn declared_types_shown(node: Node) -> Node {
	use crate::operators::Op;
	match node {
		Node::Key(target, Op::Assign, value)
			if let Node::Meta { node: name, data } = *target.clone()
				&& matches!(*data, Node::Symbol(_)) =>
		{
			Node::Key(Box::new(Node::Key(name, Op::Colon, data)), Op::Assign, Box::new(declared_types_shown(*value)))
		}
		other => other.map_children(declared_types_shown),
	}
}

/// The plain view with the positions: ["at", line, column, view] around each positioned node
pub fn positioned_plain_view(node: &Node) -> Node {
	plain(positioned_view(node))
}

fn plain(view: Node) -> Node {
	without_layers(view, &["literal", "attribute"])
}

/// The view without the layers of the `dropped` kinds, a character as a text, an empty group as ø (walk.warp
/// without_layers)
fn without_layers(view: Node, dropped: &[&str]) -> Node {
	match view {
		Node::List(mut items, bracket, separator) => match items.first().and_then(Node::text_value).as_deref() {
			Some(kind) if dropped.contains(&kind) => without_layers(items.pop().unwrap_or(Node::Empty), dropped),
			Some("list") if items.len() == 3 => Node::Empty,
			// an error and a type are opaque to a pass twin: kept whole, `class P {}` keeps its empty body and attributes
			Some("type" | "error") => Node::List(items, bracket, separator),
			Some("char") => {
				items[0] = Node::text("text");
				Node::List(items, bracket, separator)
			}
			_ => Node::List(items.into_iter().map(|item| without_layers(item, dropped)).collect(), bracket, separator),
		},
		other => other,
	}
}

/// The node a view stands for, the inverse of `node_view` (walk.warp node_of); a view no node has is an error naming
/// it, the innermost one. Warp gives a one-character text back as a character: a text is read from either
pub fn node_of_view(view: &Node) -> Node {
	node_of_handled_view(view, &[])
}

/// The node of a view from `handled_plain_view`, each data handle the node it stands for
pub fn node_of_handled_view(view: &Node, handles: &[Node]) -> Node {
	node_of(view, handles)
		.unwrap_or_else(|unreadable| crate::node::error(&format!("no node has the view {}", unreadable.serialize())))
}

/// The node, or the innermost view no node has
fn node_of(view: &Node, handles: &[Node]) -> Result<Node, Node> {
	let view = view.drop_meta();
	let Node::List(parts, ..) = view else {
		return Ok(view.clone());
	};
	let unreadable = || view.clone();
	let text = |index: usize| parts.get(index).and_then(Node::text_value).ok_or_else(unreadable);
	let node = |index: usize| parts.get(index).ok_or_else(unreadable).and_then(|part| node_of(part, handles));
	Ok(match text(0)?.as_str() {
		"symbol" => Node::Symbol(text(1)?),
		// a plain view's character is a text holding the character (plain): the parser's `"a"` too
		"text" | "char" if let Some(Node::Char(c)) = parts.get(1).map(Node::drop_meta) => Node::Char(*c),
		"text" => Node::Text(text(1)?),
		"char" => Node::Char(text(1)?.chars().next().ok_or_else(unreadable)?),
		"number" | "data" if let Some(handle) = text(1)?.strip_prefix(HANDLE_MARK) => {
			handle.parse::<usize>().ok().and_then(|index| handles.get(index)).cloned().ok_or_else(unreadable)?
		}
		"number" => crate::warp_parser::parse(&text(1)?).drop_meta().clone(),
		"key" => {
			let op = crate::operators::op_named(&text(1)?).ok_or_else(unreadable)?;
			Node::Key(Box::new(node(2)?), op, Box::new(node(3)?))
		}
		"list" => {
			let items = parts[3..].iter().map(|item| node_of(item, handles)).collect::<Result<Vec<Node>, Node>>()?;
			let bracket = bracket_of_glyphs(&text(1)?).ok_or_else(unreadable)?;
			Node::List(items, bracket, separator_of_glyph(&text(2)?).ok_or_else(unreadable)?)
		}
		"error" => Node::Error(Box::new(node(1)?)),
		"type" => Node::Type { name: Box::new(node(1)?), body: Box::new(node(2)?) },
		"attribute"
			if let Some(mark) = BRIDGED_MARKS.into_iter().find(|mark| text(1).ok().as_deref() == Some(*mark)) =>
		{
			Node::Meta { node: Box::new(node(3)?), data: Box::new(Node::key(mark, node(2)?)) }
		}
		"attribute" => node(3)?.with_attribute(&text(1)?, node(2)?),
		"literal" | "at" => node(parts.len() - 1)?,
		"template" => crate::text::interpolation::template_text(&text(1)?),
		_ => return Err(unreadable()),
	})
}

/// A node view as JSON (ø is null): warp reads it with `json(…)`, data the host parses, where the same view as a warp
/// literal is code to compile (a 100 KB chunk of sources took 2.4 GB in a debug build)
pub fn view_json(view: &Node) -> String {
	match view {
		Node::Text(text) => json_text(text),
		Node::Char(character) => json_text(&character.to_string()),
		Node::List(items, ..) => format!("[{}]", items.iter().map(view_json).collect::<Vec<_>>().join(", ")),
		Node::Empty => "null".into(),
		Node::True => "true".into(),
		Node::False => "false".into(),
		number => number.serialize(),
	}
}

pub fn json_text(text: &str) -> String {
	quoted(text, |character| character.is_control().then(|| format!("\\u{:04x}", character as u32)))
}

/// The text in double quotes, its quotes, backslashes, newlines, tabs and returns escaped, other characters by `extra`
fn quoted(text: &str, extra: impl Fn(char) -> Option<String>) -> String {
	let escaped: String = text
		.chars()
		.map(|character| match character {
			'"' | '\\' => format!("\\{character}"),
			'\n' => "\\n".into(),
			'\t' => "\\t".into(),
			'\r' => "\\r".into(),
			other => extra(other).unwrap_or_else(|| other.to_string()),
		})
		.collect();
	format!("\"{escaped}\"")
}
