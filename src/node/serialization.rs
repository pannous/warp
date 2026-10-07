//! Writing a node as wasp text: serialize and its annotation prefixes

use super::*;

impl Node {
	/// An operator in front of its one operand, ø on the left in the tree (`#x`, `-x`)
	pub(super) fn is_prefix_form(&self) -> bool {
		matches!(self.drop_meta(), Key(k, op, _) if matches!(k.drop_meta(), Empty) && writes_as_prefix(op))
	}

	pub fn serialize(&self) -> String {
		self.serialize_recurse(false)
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

	/// A key prints the annotations of its value in front of itself: `tee{@unit(cm) a:1}`
	pub(super) fn serialize_with(&self, meta: bool, attributes: bool) -> String {
		match self {
			Symbol(s) => s.clone(),
			Node::Number(n) => format!("{}", n),
			Text(t) => {
				let quote = crate::normalize::text_quote();
				format!("{quote}{t}{quote}")
			}
			Char(c) => format!("'{}'", c),
			List(nodes, bracket, separator) => {
				let close = bracket.closing();
				if nodes.is_empty() {
					format!("{}{}", bracket, close)
				} else if nodes.len() == 1 {
					format!("{}{}{}", bracket, nodes[0].serialize(), close)
				} else {
					let items: Vec<String> = nodes.iter().map(|n| n.serialize()).collect();
					let separator = separator.to_string();
					let joint = if separator.ends_with(char::is_whitespace) { separator } else { separator + " " };
					format!("{}{}{}", bracket, items.join(&joint), close)
				}
			}
			// the operand a prefix or suffix operator lacks is ø in the tree, not in the text: `#x`, `not x`, `x++`
			Key(_, op, v) if self.is_prefix_form() => {
				let space = if op.as_str().starts_with(char::is_alphabetic) { " " } else { "" };
				format!("{op}{space}{}", v.serialize_recurse(meta))
			}
			Key(k, op, v) if matches!(v.drop_meta(), Empty) && op.is_suffix() => format!("{k}{op}"),
			Key(k, op, v) if op.as_str().starts_with(char::is_alphabetic) => format!("{} {} {}", k, op, v.serialize_recurse(meta)), // 0.1 as float, not 0.1asfloat
			Key(k, Op::Colon, v) if matches!(v.drop_meta(), List(_, Bracket::Curly, _)) => format!("{}{}{}", v.attribute_prefix(), k, v.serialize_with(meta, false)), // tee{a:1}
			// `1- -x`: a prefix operand glued to the operator would read as another operator (`1--x`)
			// `g[i]` is the ungrouped `g#(i+1)` in the tree: an operation after `#` needs its parentheses back
			Key(k, Op::Hash, v) if matches!(v.drop_meta(), Key(..)) && !v.is_prefix_form() => format!("{k}#({})", v.serialize_recurse(meta)),
			Key(k, op, v) if v.is_prefix_form() => format!("{}{}{} {}", v.attribute_prefix(), k, op, v.serialize_with(meta, false)),
			Key(k, op, v) => format!("{}{}{}{}", v.attribute_prefix(), k, op, v.serialize_with(meta, false)),
			Error(e) => format!("Error({})", e.serialize_recurse(meta)),
			Empty => "ø".to_string(),
			True => "true".to_string(),
			False => "false".to_string(),
			Meta { node, data } => {
				if let Some(literal) = self.source_literal() {
					return literal.to_string();
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
			Data(d) => crate::units::describe(d).unwrap_or_else(|| format!("Data({:?})", d)),
			// _ => format!("{:?}", self),
		}
	}
}
