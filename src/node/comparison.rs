//! Comparing nodes with nodes, numbers, texts, booleans, JSON values and (native) GC objects

use super::*;

impl PartialEq for Node {
	fn eq(&self, other: &Self) -> bool {
		// metadata (positions, comments) is ignored on either side: `4` equals the parsed `4` of `[1 4 9]`
		if let (Meta { node: other_node, .. }, false) = (other, matches!(self, Meta { .. })) {
			return self == other_node.as_ref();
		}
		match self {
			True => {
				match other {
					True => true,
					False => false,
					_ => other == self, // flip symmetric cases
				}
			}
			False => {
				match other {
					True => false,
					False => true,
					_ => other == self, // flip symmetric cases
				}
			} // flip symmetric cases:
			Empty => {
				match other {
					True => false,
					False => true,
					Empty => true,
					Symbol(s) => s.is_empty(), // todo disallow empty symbol
					Text(s) => s.is_empty(),
					Node::Number(n) => n == &Number::Int(0), // ⚠️ CAREFUL
					List(l, _, _) => l.is_empty(),
					_ => self.size() == 0,
				}
			}
			Node::Number(n) => match other {
				True => !n.zero(), //  2 == true ? sUrE?? hardcore todo Truthy rules
				False => n.zero(),
				Node::Number(n2) => n == n2,
				_ => false,
			},
			Symbol(s) => {
				match other {
					True => !s.is_empty(),
					False => s.is_empty(),
					Symbol(s2) | Text(s2) => s == s2,
					_ => false,
				}
			}
			Text(s) => match other {
				True => !s.is_empty(),
				False => s.is_empty(),
				Text(s2) | Symbol(s2) => s == s2,
				_ => false,
			},

			Char(c) => match other {
				True => c != &'\0',
				False => c == &'\0',
				Char(c2) => c == c2,
				Text(c2) => *c == c2.first(),
				_ => false,
			},
			Data(d) => match other {
				Data(d2) => d == d2,
				_ => false,
			},
			Meta { node, .. } => {
				// Ignore metadata when comparing equality - unwrap both sides
				let other_unwrapped = match other {
					Meta {
						node: other_node, ..
					} => other_node.as_ref(),
					_ => other,
				};
				node.as_ref().eq(other_unwrapped)
			}
			Key(k1, op1, v1) => match other {
				Key(k2, op2, v2) => k1 == k2 && op1 == op2 && v1 == v2,
				List(items, _, _) if items.len() == 1 => other == self,
				_ => false,
			},
			List(items1, _, _) => match other {
				List(items2, _, _) => items1 == items2,
				// ignore bracket [1,2]=={1,2} and separators [1;2]==[1,2]
				Meta { node, .. } => self == node.as_ref(), // unwrap Meta
				// an object of one entry is that entry: {x:1} == x:1 (but {x} is no x)
				Key(..) => matches!(items1.as_slice(), [only] if only.drop_meta() == other),
				_ => false,
			},
			Type { name: n1, body: b1 } => match other {
				Type { name: n2, body: b2 } => n1 == n2 && b1 == b2,
				_ => false,
			},
			Error(e1) => match other {
				Error(e2) => e1 == e2,
				_ => false,
			},
		}
	}
}
impl PartialEq<str> for Node {
	fn eq(&self, other: &str) -> bool {
		match self {
			Text(s) => s == other,
			Symbol(s) => s == other,
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

impl PartialEq<i64> for Node {
	fn eq(&self, other: &i64) -> bool {
		match self {
			Node::Number(Number::Int(n)) => n == other,
			Node::Number(Number::Float(f)) => *f == *other as f64,
			Node::Number(Number::Real(r)) => r.to_f64() == *other as f64,
			// a bool acts as 1/0 under == (notes/decisions.md, card bool-type)
			True => *other == 1,
			False => *other == 0,
			Key(_, _, v) => v.as_ref().eq(other), // Compare value of Key
			Meta { node, .. } => node.as_ref().eq(other),
			Data(data) => data.downcast_ref::<crate::units::Quantity>().is_some_and(|quantity| quantity.is_amount(*other)),
			_ => false,
		}
	}
}

impl PartialEq<bool> for Node {
	fn eq(&self, other: &bool) -> bool {
		match self {
			True => *other,
			False => !*other,
			// todo 2 == true? NO only in truthy if(2) …
			Node::Number(n) => n == &if *other { 1 } else { 0 },
			// Node::Number(Number::Int(n)) => n == &if *other { 1 } else { 0 },
			// Node::Number(Number::Float(f)) => *f == if *other { 1.0 } else { 0.0 },
			Empty => !*other,
			Symbol(s) => s.is_empty() != *other,
			Text(s) => s.is_empty() != *other,
			List(l, _, _) => l.is_empty() != *other,
			Key(_, _, v) => v.is_nil() != *other, // Key is true if its value is non-empty
			_ => false,
		}
	}
}

impl PartialEq<i32> for Node {
	fn eq(&self, other: &i32) -> bool {
		self == (*other as i64)
	}
}

impl PartialEq<f64> for Node {
	fn eq(&self, other: &f64) -> bool {
		match self {
			Node::Number(Number::Float(f)) => f == other,
			Node::Number(Number::Int(n)) => *n as f64 == *other,
			Node::Number(quotient @ Number::Quotient(..)) => f64::from(*quotient) == *other,
			Node::Number(Number::BigQuotient(q)) => q.to_f64() == *other,
			// an exact real equals the f64 nearest to it, up to the rounding of evaluating it in f64
			Node::Number(Number::Real(r)) => (r.to_f64() - other).abs() <= other.abs() * 4.0 * f64::EPSILON,
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

impl PartialEq<&str> for Node {
	fn eq(&self, other: &&str) -> bool {
		match self {
			Text(s) => s == *other,
			Char(c) => *c == other.chars().next().unwrap_or('\0'),
			Symbol(s) => s == *other,
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

// Reverse comparison: &str == Node
impl PartialEq<Node> for &str {
	fn eq(&self, other: &Node) -> bool {
		other.eq(self)
	}
}

impl PartialEq<char> for Node {
	fn eq(&self, other: &char) -> bool {
		match self {
			Char(c) => c == other,
			Text(s) => {
				// Check if string is exactly one char
				let mut chars = s.chars();
				chars.next() == Some(*other) && chars.next().is_none()
			}
			Symbol(s) => {
				// Check if string is exactly one char
				let mut chars = s.chars();
				chars.next() == Some(*other) && chars.next().is_none()
			}
			Meta { node, .. } => node.as_ref().eq(other),
			_ => false,
		}
	}
}

impl PartialEq<&Node> for Node {
	fn eq(&self, other: &&Node) -> bool {
		self == *other
	}
}

// Allow &Node == primitive comparisons
impl PartialEq<i64> for &Node {
	fn eq(&self, other: &i64) -> bool {
		(*self).eq(other)
	}
}

impl PartialEq<i32> for &Node {
	fn eq(&self, other: &i32) -> bool {
		(*self).eq(other)
	}
}

impl PartialEq<f64> for &Node {
	fn eq(&self, other: &f64) -> bool {
		(*self).eq(other)
	}
}

impl PartialEq<bool> for &Node {
	fn eq(&self, other: &bool) -> bool {
		(*self).eq(other)
	}
}

// Reverse: bool == Node and bool == &Node
impl PartialEq<Node> for bool {
	fn eq(&self, other: &Node) -> bool {
		other.eq(self)
	}
}

impl PartialEq<&Node> for bool {
	fn eq(&self, other: &&Node) -> bool {
		(*other).eq(self)
	}
}

impl PartialEq<char> for &Node {
	fn eq(&self, other: &char) -> bool {
		(*self).eq(other)
	}
}

// Note: &str comparison works via blanket impl: impl<A,B> PartialEq<&B> for &A where A: PartialEq<B>
// Since Node implements PartialEq<str>, &Node automatically gets PartialEq<&str>

// Allow Box<Node> comparisons with str (for Key nodes with Symbol/Text keys)
impl PartialEq<str> for Box<Node> {
	fn eq(&self, other: &str) -> bool {
		self.as_ref().eq(other)
	}
}

impl PartialEq<&str> for Box<Node> {
	fn eq(&self, other: &&str) -> bool {
		self.as_ref().eq(*other)
	}
}

impl PartialOrd<i32> for Node {
	fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
		match self {
			Node::Number(Number::Int(n)) => (*n as i32).partial_cmp(other),
			Node::Number(Number::Float(f)) => (*f as i32).partial_cmp(other),
			Meta { node, .. } => node.as_ref().partial_cmp(other),
			_ => None,
		}
	}
}

impl PartialOrd<i64> for Node {
	fn partial_cmp(&self, other: &i64) -> Option<std::cmp::Ordering> {
		match self {
			Node::Number(Number::Int(n)) => n.partial_cmp(other),
			Node::Number(Number::Float(f)) => (*f as i64).partial_cmp(other),
			Meta { node, .. } => node.as_ref().partial_cmp(other),
			_ => None,
		}
	}
}

impl PartialOrd<f64> for Node {
	fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
		match self {
			Node::Number(Number::Int(n)) => (*n as f64).partial_cmp(other),
			Node::Number(Number::Float(f)) => f.partial_cmp(other),
			Meta { node, .. } => node.as_ref().partial_cmp(other),
			_ => None,
		}
	}
}

// PartialEq with serde_json::Value for primitive types
impl PartialEq<serde_json::Value> for Node {
	fn eq(&self, other: &serde_json::Value) -> bool {
		use serde_json::Value;

		// Fully unwrap all nested Meta nodes (consistent with Node::eq behavior)
		let mut self_unwrapped = self;
		while let Meta { node, .. } = self_unwrapped {
			self_unwrapped = node.as_ref();
		}

		match (self_unwrapped, other) {
			// Null comparison
			(Empty, Value::Null) => true,

			// Boolean comparisons
			(True, Value::Bool(true)) => true,
			(False, Value::Bool(false)) => true,

			// Number comparisons
			(Node::Number(Number::Int(n)), Value::Number(json_n)) => json_n.as_i64() == Some(*n),
			(Node::Number(Number::Float(f)), Value::Number(json_n)) => json_n.as_f64() == Some(*f),

			// String comparisons (Text, Symbol, Char all map to JSON strings)
			(Text(s), Value::String(json_s)) => s == json_s,
			(Symbol(s), Value::String(json_s)) => s == json_s,
			(Char(c), Value::String(json_s)) => &c.to_string() == json_s,

			// List comparison (arrays and objects)
			// Non-curly lists map to arrays
			(List(items, bracket, _), Value::Array(json_arr)) if !matches!(bracket, Bracket::Curly) => {
				items.len() == json_arr.len()
					&& items
						.iter()
						.zip(json_arr.iter())
						.all(|(node, json_val)| node == json_val)
			}
			(List(items, bracket, _), Value::Object(json_obj)) => {
				// Curly lists map to objects
				if matches!(bracket, Bracket::Curly) {
					// Compare as object: each item should be a Key node
					if items.len() != json_obj.len() {
						return false;
					}
					items.iter().all(|item| {
						if let Key(k, _, v) = item {
							if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
								json_obj
									.get(key_str.as_str())
									.is_some_and(|json_val| v.as_ref() == json_val)
							} else {
								false
							}
						} else {
							false
						}
					})
				} else {
					false
				}
			}

			// Key comparison (single-key objects)
			(Key(k, _, v), Value::Object(json_obj)) => {
				if let Symbol(key_str) | Text(key_str) = k.drop_meta() {
					json_obj.len() == 1
						&& json_obj
							.get(key_str.as_str())
							.is_some_and(|json_val| v.as_ref() == json_val)
				} else {
					false
				}
			}

			// All other combinations are not equal
			_ => false,
		}
	}
}

// Reverse comparison: serde_json::Value == Node
impl PartialEq<Node> for serde_json::Value {
	fn eq(&self, other: &Node) -> bool {
		other == self
	}
}

/// Trait for types that can be compared with Node::Data(GcObject)
/// Used by wasm_struct! macro to enable `is!("code", struct_value)` comparisons
#[cfg(feature = "native")]
pub trait GcComparable {
	/// Try to create Self from a GcObject
	fn try_from_gc(gc_obj: &crate::gc_traits::GcObject) -> Option<Self> where Self: Sized;
	/// Compare self with another instance
	fn gc_eq(&self, other: &Self) -> bool;
}

#[cfg(feature = "native")]
impl Node {
	/// Compare with a GcComparable type by extracting from Data variant
	pub fn eq_gc<T: GcComparable + fmt::Debug>(&self, other: &T) -> bool {
		if let Data(dada) = self {
			if let Some(gc_obj) = dada.downcast_ref::<crate::gc_traits::GcObject>() {
				if let Some(extracted) = T::try_from_gc(gc_obj) {
					return other.gc_eq(&extracted);
				}
			}
		}
		false
	}
}

/// Blanket impl: Node can be compared with any GcComparable type
/// This enables `assert_eq!(result, alice)` where alice is a wasm_struct! type
#[cfg(feature = "native")]
impl<T: GcComparable + fmt::Debug> PartialEq<T> for Node {
	fn eq(&self, other: &T) -> bool {
		self.eq_gc(other)
	}
}
