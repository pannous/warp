//! Indexing a node: node[0], node["key"], node['c'], also mutably

use super::*;

impl Index<usize> for Node {
	type Output = Node;

	fn index(&self, i: usize) -> &Self::Output {
		match self {
			List(elements, _, _) => elements.get(i).unwrap_or(&Empty),
			Key(_, _, v) => &v[i], // Pass through to value: person:{x y}[0] => x
			Meta { node, .. } => &node[i],
			_ => &Empty,
		}
	}
}

impl Index<&String> for Node {
	type Output = Node;

	fn index(&self, i: &String) -> &Self::Output {
		match self {
			List(nodes, _, _) => {
				if let Some(found) = nodes.find2(&|node| match node.drop_meta() {
					Key(k, _, _) => matches!(k.drop_meta(), Symbol(key) | Text(key) if key == i),
					Text(t) => *t == *i,
					_ => false,
				}) {
					// If we found a Key, return its value instead of the whole Key
					match found.drop_meta() {
						Key(_, _, v) => v.as_ref(),
						other => other,
					}
				} else {
					&Empty
				}
			}
			Key(k, _, v) => match k.drop_meta() {
				Symbol(key) | Text(key) if key == i => v.as_ref(),
				_ => &v[i], // Pass through to value
			},
			Meta { node, data } => {
				if node[i] != Empty {
					&node[i]
				} else {
					&data[i]
				}
			}
			_ => &Empty,
		}
	}
}

// bob['a'] -> bob["a"] supperfluous but convenient
impl Index<&char> for Node {
	type Output = Node;
	fn index(&self, i: &char) -> &Self::Output {
		self.index(i.to_string().as_str())
	}
}

impl Index<&str> for Node {
	type Output = Node;

	fn index(&self, i: &str) -> &Self::Output {
		if let Some(attribute_name) = i.strip_prefix(ATTRIBUTE_MARK) {
			return self.attribute(attribute_name).unwrap_or(&Empty);
		}
		match self {
			List(nodes, _, _) => {
				// First, search directly in this list
				if let Some(found) = nodes.find2(&|node| match node.drop_meta() {
					Key(k, _, _) => matches!(k.drop_meta(), Symbol(key) | Text(key) if key == i),
					Text(t) => t == i,
					_ => false,
				}) {
					// If we found a Key, return its value instead of the whole Key
					match found.drop_meta() {
						Key(_, _, v) => v.as_ref(),
						other => other,
					}
				} else {
					// For tag structures like ((name params...) body), search in params
					// Pattern: first element is a list containing [symbol, params...]
					if let Some(first) = nodes.first() {
						let first = first.drop_meta();
						if let List(inner, _, _) = first {
							// Skip the first element (tag name) and search in params
							for param in inner.iter().skip(1) {
								let result = &param[i];
								if result != &Empty {
									return result;
								}
							}
						}
					}
					&Empty
				}
			}
			Key(k, _, v) => match k.drop_meta() {
				Symbol(key) | Text(key) if key == i => v.as_ref(),
				_ => &v[i], // Pass through to value: person:{name:"Joe"}["name"] => "Joe"
			},
			Meta { node, data } => {
				if node[i] != Empty {
					&node[i]
				} else {
					&data[i]
				}
			}
			_ => &Empty,
		}
	}
}

impl Index<char> for Node {
	type Output = Node;

	fn index(&self, i: char) -> &Self::Output {
		self.index(i.to_string().as_str())
	}
}

impl IndexMut<usize> for Node {
	fn index_mut(&mut self, i: usize) -> &mut Self::Output {
		match self {
			List(elements, _, _) => {
				if i < elements.len() {
					&mut elements[i]
				} else {
					panic!("Index out of bounds")
				}
			}
			Meta { node, .. } => &mut node[i],
			_ => panic!("Cannot mutably index this node type"),
		}
	}
}

impl IndexMut<&String> for Node {
	fn index_mut(&mut self, i: &String) -> &mut Self::Output {
		if let Some(attribute_name) = i.strip_prefix(ATTRIBUTE_MARK) {
			return self.attribute_slot(attribute_name);
		}
		match self {
			List(nodes, _, _) => {
				if let Some(found) = nodes.iter_mut().find(|node| match node.drop_meta() {
					Key(k, _, _) => matches!(k.drop_meta(), Symbol(key) | Text(key) if key == i),
					Text(t) => t == i,
					_ => false,
				}) {
					// If we found a Key, return mutable reference to its value
					// Need to unwrap Meta first if present
					match found {
						Meta { node, .. } => match node.as_mut() {
							Key(_, _, v) => v.as_mut(),
							other => other,
						},
						Key(_, _, v) => v.as_mut(),
						other => other,
					}
				} else {
					panic!("Key '{}' not found", i)
				}
			}
			Key(_, _, v) => &mut v[i], // Pass through to value
			Meta { node, .. } => &mut node[i],
			_ => panic!("Cannot mutably index this node type"),
		}
	}
}

impl IndexMut<&str> for Node {
	fn index_mut(&mut self, i: &str) -> &mut Self::Output {
		&mut self[&i.to_string()]
	}
}

impl IndexMut<char> for Node {
	fn index_mut(&mut self, i: char) -> &mut Self::Output {
		&mut self[&i.to_string()]
	}
}
