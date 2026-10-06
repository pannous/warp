//! Conversions: not, and nodes from and to Rust values

use super::*;

// Implement Not for owned Node - returns Node for compatibility with existing tests
impl Not for Node {
	type Output = Node;

	fn not(self) -> Self::Output {
		if self.to_bool() {
			False
		} else {
			True
		}
	}
}

// Implement Not for &Node to support !&node["key"] syntax - returns bool
impl Not for &Node {
	type Output = bool;

	fn not(self) -> Self::Output {
		!self.to_bool()
	}
}

// From trait implementations for automatic conversion in assignments
impl From<&str> for Node {
	fn from(s: &str) -> Self {
		Text(s.to_string())
	}
}

impl From<String> for Node {
	fn from(s: String) -> Self {
		Text(s)
	}
}

impl From<i32> for Node {
	fn from(n: i32) -> Self {
		Node::Number(Number::Int(n as i64))
	}
}

impl From<i64> for Node {
	fn from(n: i64) -> Self {
		Node::Number(Number::Int(n))
	}
}

impl From<f32> for Node {
	fn from(n: f32) -> Self {
		Node::Number(Number::Float(n as f64))
	}
}

impl From<f64> for Node {
	fn from(n: f64) -> Self {
		Node::Number(Number::Float(n))
	}
}

impl From<bool> for Node {
	fn from(b: bool) -> Self {
		if b {
			True
		} else {
			False
		}
	}
}

impl From<char> for Node {
	fn from(c: char) -> Self {
		Char(c)
	}
}

// Allow Node to be converted to bool via .into() or bool::from()
impl From<Node> for bool {
	fn from(node: Node) -> Self {
		node.to_bool()
	}
}

impl From<&Node> for bool {
	fn from(node: &Node) -> Self {
		node.to_bool()
	}
}

// ============ Arithmetic Operators ============
