// LIST AND VECTOR EXTENSIONS

use crate::node::Node;

// [a,b,c] => [(0,a), (1,b), (2,c)] aka enumerate
pub trait Indexed<T> {
	fn indexed(self) -> impl Iterator<Item = (usize, T)>;
}

impl<T> Indexed<T> for Vec<T> {
	fn indexed(self) -> impl Iterator<Item = (usize, T)> {
		self.into_iter()
			// .map(|item| item.into_result())
			.enumerate()
	}
}

// Implement Indexed for Node to support enumerating List nodes
impl Indexed<Node> for Node {
	fn indexed(self) -> impl Iterator<Item = (usize, Node)> {
		match self {
			Node::List(items, _, _) => items.into_iter().enumerate(),
			_ => vec![].into_iter().enumerate(),
		}
	}
}


pub trait Filter<T> {
	fn filter(self, f: fn(&T) -> bool) -> Vec<T>;
}

impl<T> Filter<T> for Vec<T> {
	fn filter(self, f: fn(&T) -> bool) -> Vec<T> {
		self.into_iter().filter(f).collect()
	}
}

// Implement Filter for Node to support filtering List nodes
impl Filter<Node> for Node {
	fn filter(self, f: fn(&Node) -> bool) -> Vec<Node> {
		match self {
			Node::List(items, _, _) => items.into_iter().filter(f).collect(),
			_ => vec![],
		}
	}
}

pub(crate) trait VecExtensions2<T: Clone> {
	fn find2(&self, f: &dyn Fn(&T) -> bool) -> Option<&T>;
}

impl<T: Clone> VecExtensions2<T> for Vec<T> {
	fn find2(&self, f: &dyn Fn(&T) -> bool) -> Option<&T> {
		self.iter().find(|&x| f(x))
	}
}
pub fn map<T, U, F>(items: Vec<T>, func: F) -> Vec<U>
where
	F: FnMut(T) -> U,
{
	items.into_iter().map(func).collect()
}
