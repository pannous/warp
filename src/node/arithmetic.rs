//! Node arithmetic: + - * / between nodes and numbers; mismatched kinds give an error value

use super::*;

/// `left op right` on the nodes inside one Meta wrapper each; the result keeps the left operand's metadata
fn keeping_left_meta(left: &Node, right: &Node, compute: impl FnOnce(&Node, &Node) -> Node) -> Node {
	let right = match right {
		Meta { node, .. } => node.as_ref(),
		_ => right,
	};
	match left {
		Meta { node, data } => Meta { node: Box::new(compute(node, right)), data: data.clone() },
		_ => compute(left, right),
	}
}

impl Add<&Node> for &Node {
	type Output = Node;
	fn add(self, rhs: &Node) -> Node {
		keeping_left_meta(self, rhs, |left, right| match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 + *n2),
			(True, True) => Node::Number(Number::Int(2)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) + *n),
			(Node::Number(n), True) => Node::Number(*n + Number::Int(1)),
			(False, Node::Number(n)) | (Node::Number(n), False) => Node::Number(*n),
			(Empty, Node::Number(n)) | (Node::Number(n), Empty) => Node::Number(*n),
			_ => error(&format!("Cannot add {left:?} and {right:?}")),
		})
	}
}

impl Sub<&Node> for &Node {
	type Output = Node;
	fn sub(self, rhs: &Node) -> Node {
		keeping_left_meta(self, rhs, |left, right| match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 - *n2),
			(True, True) => Node::Number(Number::Int(0)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) - *n),
			(Node::Number(n), True) => Node::Number(*n - Number::Int(1)),
			(Node::Number(n), False) => Node::Number(*n),
			(False, Node::Number(n)) => Node::Number(Number::Int(0) - *n),
			(Empty, Node::Number(n)) => Node::Number(Number::Int(0) - *n),
			(Node::Number(n), Empty) => Node::Number(*n),
			_ => error(&format!("Cannot subtract {left:?} and {right:?}")),
		})
	}
}

impl Mul<&Node> for &Node {
	type Output = Node;
	fn mul(self, rhs: &Node) -> Node {
		keeping_left_meta(self, rhs, |left, right| match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 * *n2),
			(True, Node::Number(n)) | (Node::Number(n), True) => Node::Number(*n),
			(False, _) | (_, False) => Node::Number(Number::Int(0)),
			(Empty, _) | (_, Empty) => Node::Number(Number::Int(0)),
			_ => error(&format!("Cannot multiply {left:?} and {right:?}")),
		})
	}
}

impl Div<&Node> for &Node {
	type Output = Node;
	fn div(self, rhs: &Node) -> Node {
		keeping_left_meta(self, rhs, |left, right| match (left, right) {
			(Node::Number(n1), Node::Number(n2)) => Node::Number(*n1 / *n2),
			(Node::Number(n), True) => Node::Number(*n / Number::Int(1)),
			(True, Node::Number(n)) => Node::Number(Number::Int(1) / *n),
			(False, Node::Number(_)) => Node::Number(Number::Int(0)),
			(Empty, Node::Number(_)) => Node::Number(Number::Int(0)),
			_ => error(&format!("Cannot divide {left:?} and {right:?}")),
		})
	}
}

/// `node op 3`, `node op 2.5` and `3 op node` through the node operator
macro_rules! scalar_operands {
	($($trait:ident $method:ident),*) => {$(
		impl $trait<i64> for &Node {
			type Output = Node;
			fn $method(self, rhs: i64) -> Node { <&Node as $trait<&Node>>::$method(self, &Node::int(rhs)) }
		}
		impl $trait<i32> for &Node {
			type Output = Node;
			fn $method(self, rhs: i32) -> Node { <&Node as $trait<&Node>>::$method(self, &Node::int(rhs as i64)) }
		}
		impl $trait<f64> for &Node {
			type Output = Node;
			fn $method(self, rhs: f64) -> Node { <&Node as $trait<&Node>>::$method(self, &Node::float(rhs)) }
		}
		impl $trait<&Node> for i64 {
			type Output = Node;
			fn $method(self, rhs: &Node) -> Node { <&Node as $trait<&Node>>::$method(&Node::int(self), rhs) }
		}
		impl $trait<&Node> for i32 {
			type Output = Node;
			fn $method(self, rhs: &Node) -> Node { <&Node as $trait<&Node>>::$method(&Node::int(self as i64), rhs) }
		}
		impl $trait<&Node> for f64 {
			type Output = Node;
			fn $method(self, rhs: &Node) -> Node { <&Node as $trait<&Node>>::$method(&Node::float(self), rhs) }
		}
	)*};
}

scalar_operands!(Add add, Sub sub, Mul mul, Div div);
