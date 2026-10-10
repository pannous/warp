//! A node as JSON or XML, and from JSON

use super::*;
use serde_json::{Map, Value};

impl Node {
	pub fn to_json(&self) -> Result<String, serde_json::Error> {
		let value = self.to_json_value();
		serde_json::to_string_pretty(&value)
	}

	pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
		let value = self.to_json_value();
		serde_json::to_string(&value)
	}

	/// Convert Node to XML string representation
	/// Key nodes become XML tags, dotted keys (.attr) become attributes
	pub fn to_xml(&self) -> String {
		match self.drop_meta() {
			Key(tag_name, _, body) => {
				let mut attributes = String::new();
				let mut content = Vec::new();
				match body.as_ref() {
					List(items, _, _) => {
						for item in items {
							match xml_attribute(item) {
								Some(attribute) => attributes += &format!(" {attribute}"),
								None => content.push(item.to_xml()),
							}
						}
					}
					Empty => {}
					single => content.push(single.to_xml()),
				}
				if content.is_empty() {
					format!("<{tag_name}{attributes} />")
				} else {
					format!("<{tag_name}{attributes}>{}</{tag_name}>", content.concat())
				}
			}
			Text(s) | Symbol(s) => s.clone(),
			List(items, _, _) => items.iter().map(Node::to_xml).collect(),
			Empty => String::new(),
			_ => self.serialize(),
		}
	}

	pub(super) fn to_json_value(&self) -> Value {
		match self {
			True => Value::Bool(true),
			False => Value::Bool(false),
			Empty => Value::Null,
			Node::Number(Number::Int(n)) => Value::Number((*n).into()),
			Node::Number(Number::Float(f)) => serde_json::Number::from_f64(*f).map(Value::Number).unwrap_or(Value::Null),
			Node::Number(n) => Value::String(n.to_string()),
			Text(s) | Symbol(s) => Value::String(s.clone()),
			Char(c) => Value::String(c.to_string()),
			// curly braces are an object, the items without a key numbered by their place; others an array
			List(items, Bracket::Curly, _) => {
				let mut map = Map::new();
				for item in items {
					match item {
						Key(..) => insert_json_entry(&mut map, item),
						List(nested, Bracket::Curly, _) => nested.iter().for_each(|entry| insert_json_entry(&mut map, entry)),
						other => {
							map.insert(map.len().to_string(), other.to_json_value());
						}
					}
				}
				Value::Object(map)
			}
			List(items, _, _) => Value::Array(items.iter().map(Node::to_json_value).collect()),
			Key(..) => {
				let mut map = Map::new();
				insert_json_entry(&mut map, self);
				Value::Object(map)
			}
			Data(d) => json_object([("_type", Value::String(d.type_name.clone()))]),
			Meta { node, data } => match data.as_ref() {
				// metadata keys as dotted keys, other metadata as .meta
				List(items, ..) if items.iter().any(|item| matches!(item, Key(..))) => {
					let mut map: Map<String, Value> = items.iter().filter_map(|item| match item {
						Key(key, _, value) => Some((format!(".{key}"), value.to_json_value())),
						_ => None,
					}).collect();
					map.insert("_value".to_string(), node.to_json_value());
					Value::Object(map)
				}
				List(..) => json_object([(".meta", data.to_json_value()), ("_value", node.to_json_value())]),
				_ if data.get_lineinfo().is_some() || **data == Empty => node.to_json_value(),
				_ => match node.to_json_value() {
					Value::Object(mut map) => {
						map.insert(".meta".to_string(), data.to_json_value());
						Value::Object(map)
					}
					inner => json_object([("_value", inner), (".meta", data.to_json_value())]),
				},
			},
			Type { name, body } => json_object([("_type", name.to_json_value()), ("fields", body.to_json_value())]),
			Error(e) => json_object([("_error", e.to_json_value())]),
		}
	}

	pub fn from_json(json: &str) -> Result<Node, serde_json::Error> {
		serde_json::from_str(json)
	}
}

/// `.name=value` of an item `.name: value` (a flag `.name: true` is just the name); None for content
fn xml_attribute(item: &Node) -> Option<String> {
	let Key(key, _, value) = item.drop_meta() else { return None };
	let (Symbol(key) | Text(key)) = key.drop_meta() else { return None };
	let name = key.strip_prefix('.')?;
	Some(match value.as_ref() {
		True => name.to_string(),
		Text(s) | Symbol(s) => format!("{name}=\"{s}\""),
		Number(n) => format!("{name}=\"{n}\""),
		other => format!("{name}=\"{}\"", other.serialize()),
	})
}

/// The entry `key: value` with a text or symbol key into the object; any other node adds nothing
fn insert_json_entry(map: &mut Map<String, Value>, entry: &Node) {
	if let Key(key, _, value) = entry {
		if let Symbol(key) | Text(key) = key.drop_meta() {
			map.insert(key.clone(), value.to_json_value());
		}
	}
}

fn json_object<const N: usize>(entries: [(&str, Value); N]) -> Value {
	Value::Object(entries.into_iter().map(|(key, value)| (key.to_string(), value)).collect())
}
