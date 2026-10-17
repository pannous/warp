//! Writing a node as warp text: serialize and its annotation prefixes

use super::*;

/// How a bool prints (user, card bool-type): yes and no; true and false stay accepted as input
pub const YES: &str = "yes";
pub const NO: &str = "no";

/// A text in quotes, a quote or `\` in it escaped by a `\` so it reads back, as the runtime prints it (P126)
fn quoted(text: &str) -> String {
	quoted_by(text, crate::normalize::text_quote())
}

fn quoted_by(text: &str, quote: char) -> String {
	let mut written = String::from(quote);
	for c in text.chars() {
		if c == quote || c == '\\' {
			written.push('\\');
		}
		written.push(c);
	}
	written.push(quote);
	written
}

impl Node {
	/// An operator in front of its one operand, ø on the left in the tree (`#x`, `-x`)
	pub(super) fn is_prefix_form(&self) -> bool {
		matches!(self.drop_meta(), Key(k, op, _) if k.is_empty_node() && writes_as_prefix(op))
	}

	pub fn serialize(&self) -> String {
		self.serialize_recurse(false)
	}

	/// A list item: one character in single quotes, a one-character text too, as the runtime prints it
	/// (chars-codepoints: `['a' "bc"]`); in code without brackets in double quotes, as the hint `fail "?"` writes it
	fn item_text(&self, bracket: &Bracket) -> String {
		match self.drop_meta() {
			Char(c) if *bracket == Bracket::None => quoted(&c.to_string()),
			Text(t) if t.chars().count() == 1 && *bracket != Bracket::None => quoted_by(t, '\''),
			_ => self.serialize(),
		}
	}

	pub fn meta_string(&self) -> String {
		// todo as impl for Meta?
		// Extract MetaData from Data node if present
		if self["comment"] != Empty {
			return format!("/* {} */", self["comment"]);
		}
		if let Data(dada) = self {
			if let Some(_info) = dada.downcast_ref::<LineInfo>() {
			} else {
				return format!("{:?}", dada);
			}
		}
		"".to_string()
	}

	pub fn serialize_recurse(&self, meta: bool) -> String {
		self.serialize_with(meta, true)
	}

	pub(super) fn attribute_source((name, value): (&str, &Node)) -> String {
		match value {
			True => format!("{ATTRIBUTE_MARK}{name} "),
			_ => format!("{ATTRIBUTE_MARK}{name}({}) ", value.serialize()),
		}
	}

	/// `@name(value)` annotations in front of the node, outermost first, each followed by a space
	pub(super) fn attribute_prefix(&self) -> String {
		self.attributes().into_iter().map(Self::attribute_source).collect()
	}

	pub(super) fn own_attribute_prefix(&self) -> String {
		self.own_attribute().map(Self::attribute_source).unwrap_or_default()
	}

	/// The left side of `op`: a text or character operand keeps its quotes (`"ab"+x`), a key of `:` stays bare (`name: 1`)
	fn key_text(&self, op: &Op, meta: bool) -> String {
		if *op == Op::Colon {
			self.to_string()
		} else {
			self.operand_text(meta)
		}
	}

	fn operand_text(&self, meta: bool) -> String {
		match self.drop_meta() {
			Text(_) | Char(_) => self.serialize_recurse(meta),
			_ => self.to_string(),
		}
	}

	/// A key prints the annotations of its value in front of itself: `tee{@unit(cm) a:1}`
	pub(super) fn serialize_with(&self, meta: bool, attributes: bool) -> String {
		match self {
			Symbol(s) => s.clone(),
			Node::Number(n) => format!("{}", n),
			Text(t) => quoted(t),
			Char(c) => quoted_by(&c.to_string(), '\''),
			List(nodes, bracket, separator) => {
				let items: Vec<String> = nodes.iter().map(|node| node.item_text(bracket)).collect();
				format!("{bracket}{}{}", items.join(&separator.joint()), bracket.closing())
			}
			// the operand a prefix or suffix operator lacks is ø in the tree, not in the text: `#x`, `not x`, `x++`
			Key(_, op, v) if self.is_prefix_form() => {
				let space = if op.as_str().starts_with(char::is_alphabetic) { " " } else { "" };
				format!("{op}{space}{}", v.serialize_recurse(meta))
			}
			Key(k, op, v) if v.is_empty_node() && op.is_suffix() => {
				format!("{}{op}", k.operand_text(meta))
			}
			Key(k, op, v) if op.as_str().starts_with(char::is_alphabetic) => {
				format!("{} {} {}", k.operand_text(meta), op, v.serialize_recurse(meta))
			} // 0.1 as float, not 0.1asfloat
			Key(k, Op::Colon, v) if matches!(v.drop_meta(), List(_, Bracket::Curly, _)) => {
				format!("{}{}{}", v.attribute_prefix(), k, v.serialize_with(meta, false))
			} // tee{a:1}
			// `1- -x`: a prefix operand glued to the operator would read as another operator (`1--x`)
			// `g[i]` is the ungrouped `g#(i+1)` in the tree: an operation after `#` needs its parentheses back
			Key(k, Op::Hash, v) if matches!(v.drop_meta(), Key(..)) && !v.is_prefix_form() => {
				format!("{}#({})", k.operand_text(meta), v.serialize_recurse(meta))
			}
			Key(k, op, v) => {
				let space = if v.is_prefix_form() { " " } else { "" };
				format!("{}{}{op}{space}{}", v.attribute_prefix(), k.key_text(op, meta), v.serialize_with(meta, false))
			}
			Error(e) => format!("Error({})", e.serialize_recurse(meta)),
			Empty => "ø".to_string(),
			True => YES.to_string(),
			False => NO.to_string(),
			Meta { node, data } => {
				if let Some(literal) = self.source_literal() {
					return literal.to_string();
				}
				if let Some(name) = self.type_value_name() {
					return format!("class {name}");
				}
				let own_prefix = if attributes { self.own_attribute_prefix() } else { String::new() };
				let inner = format!("{own_prefix}{}", node.serialize_with(meta, attributes));
				if meta {
					format!("{} {}", inner, data.meta_string())
				} else {
					inner
				}
			}
			Type { name, body } => format!("type {} {}", name.serialize_recurse(meta), body.serialize_recurse(meta)),
			Data(d) => crate::numbers::units::describe(d).unwrap_or_else(|| format!("Data({:?})", d)),
			// _ => format!("{:?}", self),
		}
	}
}
