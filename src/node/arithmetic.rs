//! Node arithmetic: + - * / between nodes and numbers; mismatched kinds give an error value

use super::*;

impl Add<&Node> for &Node {
	type Output = Node;

	fn add(self, rhs: &Node) -> Self::Output {
		let (left, left_meta) = match self {
			Meta { node, data } => (node.as_ref(), Some(data)),
			_ => (self, None),
		};
		let right = match rhs {
			Meta { node, .. } => node.as_ref(),
			_ => rhs,
		};

		// Match on types and compute
		let result = match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 + *n2),
			(True, True) => Node::Number(Number::Int(2)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) + *n),
			(Node::Number(n), True) => Node::Number(*n + Number::Int(1)),
			(False, Node::Number(n)) | (Node::Number(n), False) => Node::Number(*n),
			(Empty, Node::Number(n)) | (Node::Number(n), Empty) => Node::Number(*n),
			_ => error(&format!("Cannot add {left:?} and {right:?}")),
		};

		// Preserve metadata from left operand
		if let Some(data) = left_meta {
			Meta {
				node: Box::new(result),
				data: (*data).clone(),
			}
		} else {
			result
		}
	}
}

impl Add<i64> for &Node {
	type Output = Node;
	fn add(self, rhs: i64) -> Self::Output {
		self + &Node::int(rhs)
	}
}

impl Add<f64> for &Node {
	type Output = Node;
	fn add(self, rhs: f64) -> Self::Output {
		self + &Node::float(rhs)
	}
}

impl Add<i32> for &Node {
	type Output = Node;
	fn add(self, rhs: i32) -> Self::Output {
		self + &Node::int(rhs as i64)
	}
}

impl Add<&Node> for i64 {
	type Output = Node;
	fn add(self, rhs: &Node) -> Self::Output {
		&Node::int(self) + rhs
	}
}

impl Add<&Node> for f64 {
	type Output = Node;
	fn add(self, rhs: &Node) -> Self::Output {
		&Node::float(self) + rhs
	}
}

impl Add<&Node> for i32 {
	type Output = Node;
	fn add(self, rhs: &Node) -> Self::Output {
		&Node::int(self as i64) + rhs
	}
}

// Sub implementations
impl Sub<&Node> for &Node {
	type Output = Node;

	fn sub(self, rhs: &Node) -> Self::Output {
		let (left, left_meta) = match self {
			Meta { node, data } => (node.as_ref(), Some(data)),
			_ => (self, None),
		};
		let right = match rhs {
			Meta { node, .. } => node.as_ref(),
			_ => rhs,
		};

		// Match on types and compute
		let result = match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 - *n2),
			(True, True) => Node::Number(Number::Int(0)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) - *n),
			(Node::Number(n), True) => Node::Number(*n - Number::Int(1)),
			(Node::Number(n), False) => Node::Number(*n),
			(False, Node::Number(n)) => Node::Number(Number::Int(0) - *n),
			(Empty, Node::Number(n)) => Node::Number(Number::Int(0) - *n),
			(Node::Number(n), Empty) => Node::Number(*n),
			_ => error(&format!("Cannot subtract {left:?} and {right:?}")),
		};

		// Preserve metadata from left operand
		if let Some(data) = left_meta {
			Meta {
				node: Box::new(result),
				data: (*data).clone(),
			}
		} else {
			result
		}
	}
}

impl Sub<i64> for &Node {
	type Output = Node;
	fn sub(self, rhs: i64) -> Self::Output {
		self - &Node::int(rhs)
	}
}

impl Sub<f64> for &Node {
	type Output = Node;
	fn sub(self, rhs: f64) -> Self::Output {
		self - &Node::float(rhs)
	}
}

impl Sub<i32> for &Node {
	type Output = Node;
	fn sub(self, rhs: i32) -> Self::Output {
		self - &Node::int(rhs as i64)
	}
}

impl Sub<&Node> for i64 {
	type Output = Node;
	fn sub(self, rhs: &Node) -> Self::Output {
		&Node::int(self) - rhs
	}
}

impl Sub<&Node> for f64 {
	type Output = Node;
	fn sub(self, rhs: &Node) -> Self::Output {
		&Node::float(self) - rhs
	}
}

impl Sub<&Node> for i32 {
	type Output = Node;
	fn sub(self, rhs: &Node) -> Self::Output {
		&Node::int(self as i64) - rhs
	}
}

// Mul implementations
impl Mul<&Node> for &Node {
	type Output = Node;

	fn mul(self, rhs: &Node) -> Self::Output {
		let (left, left_meta) = match self {
			Meta { node, data } => (node.as_ref(), Some(data)),
			_ => (self, None),
		};
		let right = match rhs {
			Meta { node, .. } => node.as_ref(),
			_ => rhs,
		};

		// Match on types and compute
		let result = match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 * *n2),
			(True, Node::Number(n)) | (Node::Number(n), True) => Node::Number(*n),
			(False, _) | (_, False) => Node::Number(Number::Int(0)),
			(Empty, _) | (_, Empty) => Node::Number(Number::Int(0)),
			_ => error(&format!("Cannot multiply {left:?} and {right:?}")),
		};

		// Preserve metadata from left operand
		if let Some(data) = left_meta {
			Meta {
				node: Box::new(result),
				data: (*data).clone(),
			}
		} else {
			result
		}
	}
}

impl Mul<i64> for &Node {
	type Output = Node;
	fn mul(self, rhs: i64) -> Self::Output {
		self * &Node::int(rhs)
	}
}

impl Mul<f64> for &Node {
	type Output = Node;
	fn mul(self, rhs: f64) -> Self::Output {
		self * &Node::float(rhs)
	}
}

impl Mul<i32> for &Node {
	type Output = Node;
	fn mul(self, rhs: i32) -> Self::Output {
		self * &Node::int(rhs as i64)
	}
}

impl Mul<&Node> for i64 {
	type Output = Node;
	fn mul(self, rhs: &Node) -> Self::Output {
		&Node::int(self) * rhs
	}
}

impl Mul<&Node> for f64 {
	type Output = Node;
	fn mul(self, rhs: &Node) -> Self::Output {
		&Node::float(self) * rhs
	}
}

impl Mul<&Node> for i32 {
	type Output = Node;
	fn mul(self, rhs: &Node) -> Self::Output {
		&Node::int(self as i64) * rhs
	}
}

// Div implementations
impl Div<&Node> for &Node {
	type Output = Node;

	fn div(self, rhs: &Node) -> Self::Output {
		let (left, left_meta) = match self {
			Meta { node, data } => (node.as_ref(), Some(data)),
			_ => (self, None),
		};
		let right = match rhs {
			Meta { node, .. } => node.as_ref(),
			_ => rhs,
		};

		// Match on types and compute
		let result = match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 / *n2),
			(Node::Number(n), True) => Node::Number(*n / Number::Int(1)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) / *n),
			(False, Node::Number(_)) => Node::Number(Number::Int(0)),
			(Empty, Node::Number(_)) => Node::Number(Number::Int(0)),
			_ => error(&format!("Cannot divide {left:?} and {right:?}")),
		};

		// Preserve metadata from left operand
		if let Some(data) = left_meta {
			Meta {
				node: Box::new(result),
				data: (*data).clone(),
			}
		} else {
			result
		}
	}
}

impl Div<i64> for &Node {
	type Output = Node;
	fn div(self, rhs: i64) -> Self::Output {
		self / &Node::int(rhs)
	}
}

impl Div<f64> for &Node {
	type Output = Node;
	fn div(self, rhs: f64) -> Self::Output {
		self / &Node::float(rhs)
	}
}

impl Div<i32> for &Node {
	type Output = Node;
	fn div(self, rhs: i32) -> Self::Output {
		self / &Node::int(rhs as i64)
	}
}

impl Div<&Node> for i64 {
	type Output = Node;
	fn div(self, rhs: &Node) -> Self::Output {
		&Node::int(self) / rhs
	}
}

impl Div<&Node> for f64 {
	type Output = Node;
	fn div(self, rhs: &Node) -> Self::Output {
		&Node::float(self) / rhs
	}
}

impl Div<&Node> for i32 {
	type Output = Node;
	fn div(self, rhs: &Node) -> Self::Output {
		&Node::int(self as i64) / rhs
	}
}
