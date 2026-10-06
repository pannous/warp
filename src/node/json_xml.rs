//! A node as JSON or XML, and from JSON

use super::*;

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
				let mut attributes = Vec::new();
				let mut content_parts = Vec::new();

				// Separate attributes (dotted keys) from content
				match body.as_ref() {
					List(items, _, _) => {
						for item in items {
							match item.drop_meta() {
								Key(k, _, v) => {
									if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
										if let Some(attr_name) = key_str.strip_prefix('.') {
											// This is an attribute
											// Remove leading dot
											match v.as_ref() {
												True => {
													// Boolean attribute (no value)
													attributes.push(attr_name.to_string());
												}
												Text(s) | Symbol(s) => {
													attributes
														.push(format!("{}=\"{}\"", attr_name, s));
												}
												Number(n) => {
													attributes
														.push(format!("{}=\"{}\"", attr_name, n));
												}
												_ => {
													let val = Node::serialize(v);
													attributes
														.push(format!("{}=\"{}\"", attr_name, val));
												}
											}
										} else {
											// Non-attribute key - treat as content
											content_parts.push(item.to_xml());
										}
									} else {
										// Non-string key - treat as content
										content_parts.push(item.to_xml());
									}
								}
								_ => {
									// This is content
									content_parts.push(item.to_xml());
								}
							}
						}
					}
					Empty => {
						// Empty body
					}
					other => {
						// Single content item
						content_parts.push(other.to_xml());
					}
				}

				// Build XML tag
				let attrs_str = if attributes.is_empty() {
					String::new()
				} else {
					format!(" {}", attributes.join(" "))
				};

				if content_parts.is_empty() {
					// Self-closing tag
					format!("<{}{} />", tag_name, attrs_str)
				} else {
					// Tag with content
					let content = content_parts.join("");
					format!("<{}{}>{}</{}>", tag_name, attrs_str, content, tag_name)
				}
			}
			Text(s) => s.clone(),
			Symbol(s) => s.clone(),
			List(items, _, _) => {
				// Multiple items - convert each to XML
				items
					.iter()
					.map(|item| item.to_xml())
					.collect::<Vec<_>>()
					.join("")
			}
			Empty => String::new(),
			_ => {
				// For other node types, fall back to serialize
				self.serialize()
			}
		}
	}

	pub(super) fn to_json_value(&self) -> serde_json::Value {
		use serde_json::{Map, Value};

		match self {
			True => Value::Bool(true),
			False => Value::Bool(false),
			Empty => Value::Null,
			Node::Number(Number::Int(n)) => Value::Number((*n).into()),
			Node::Number(Number::Float(f)) => serde_json::Number::from_f64(*f)
				.map(Value::Number)
				.unwrap_or(Value::Null),
			Node::Number(n) => Value::String(format!("{}", n)),
			Text(s) | Symbol(s) => Value::String(s.clone()),
			Char(c) => Value::String(c.to_string()),
			List(items, bracket, _) => {
				// Curly braces -> object with items, Square/Round -> array
				match bracket {
					Bracket::Curly => {
						let mut map = Map::new();
						for item in items {
							match item {
								Key(k, _, v) => {
									if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
										map.insert(key_str.clone(), v.to_json_value());
									}
								}
								List(nested, Bracket::Curly, _) => {
									// Nested curly lists become nested objects
									for nested_item in nested {
										if let Key(k, _, v) = nested_item {
											if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
												map.insert(key_str.clone(), v.to_json_value());
											}
										}
									}
								}
								other => {
									// let key = format!("item_{}", map.len());
									let key = format!("{}", map.len()); // just the number
									map.insert(key, other.to_json_value());
								}
							}
						}
						Value::Object(map)
					}
					_ => Value::Array(items.iter().map(|n| n.to_json_value()).collect()),
				}
			}
			Key(k, _, v) => {
				let mut map = Map::new();
				if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
					map.insert(key_str.clone(), v.to_json_value());
				}
				Value::Object(map)
			}
			Data(d) => {
				let mut map = Map::new();
				map.insert("_type".to_string(), Value::String(d.type_name.clone()));
				Value::Object(map)
			}
			Meta { node, data } => {
				// Encode metadata as dotted keys or .meta array
				if let List(items, ..) = data.as_ref() {
					let has_keys = items.iter().any(|n| matches!(n, Key(..)));

					if has_keys {
						// Extract dotted keys from metadata
						let mut map = Map::new();
						for item in items {
							if let Key(k, _, v) = item {
								map.insert(format!(".{}", k), v.to_json_value());
							}
						}
						// Add the wrapped value
						map.insert("_value".to_string(), node.to_json_value());
						Value::Object(map)
					} else {
						// Non-Key metadata: use .meta array
						let mut map = Map::new();
						map.insert(".meta".to_string(), data.to_json_value());
						map.insert("_value".to_string(), node.to_json_value());
						Value::Object(map)
					}
				} else if let Some(_info) = data.get_lineinfo() {
					node.to_json_value() // ignore lineinfo
				} else if **data != Empty {
					let inner = node.to_json_value();
					let meta_val = data.to_json_value();
					match inner {
						Value::Object(mut map) => {
							map.insert(".meta".to_string(), meta_val);
							Value::Object(map)
						}
						_ => {
							let mut map = Map::new();
							map.insert("_value".to_string(), inner);
							map.insert(".meta".to_string(), meta_val);
							Value::Object(map)
						}
					}
				} else {
					// No metadata, just unwrap
					node.to_json_value()
				}
			}
			Type { name, body } => {
				let mut map = Map::new();
				map.insert("_type".to_string(), name.to_json_value());
				map.insert("fields".to_string(), body.to_json_value());
				Value::Object(map)
			}
			Error(e) => {
				let mut map = Map::new();
				map.insert("_error".to_string(), e.to_json_value());
				Value::Object(map)
			}
		}
	}

	pub fn from_json(json: &str) -> Result<Node, serde_json::Error> {
		serde_json::from_str(json)
	}
}
