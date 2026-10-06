// LIST AND VECTOR EXTENSIONS

use crate::extensions::numbers::Number;
use crate::node::Node;

use std::ops::Range;

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

pub trait FromRange<T> {
	fn from(range: Range<T>) -> Vec<T>;
}

// impl From<Range<i32>> for Vec<i32> {
impl FromRange<i32> for Vec<i32> {
	fn from(range: Range<i32>) -> Self {
		range.collect()
	}
}


pub enum List<T> {
	Vector(Vec<T>),
	// Array([T; 5]), // makes no sense to have a list of fixed array length
}

impl From<[i32; 5]> for List<Number> {
	fn from(array: [i32; 5]) -> Self {
		let data = array.iter().map(|&x| Number::Int(x as i64)).collect();
		List::Vector(data)
	}
}



pub trait StringVecExtensions<String> {
	fn with(&self, n: &str) -> Vec<String>;
}

impl StringVecExtensions<String> for Vec<String> {
	fn with(&self, n: &str) -> Vec<String> {
		let mut v = self.clone();
		v.push(n.to_string());
		v
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
// The "From" trait is used when you want to define your own custom conversion from one type to ano
// it is used with let b:B=a.into() short form for From::from(a) or B::from(a)
struct MyVec<T>(Vec<T>);

impl<T: Clone> From<[T; 5]> for MyVec<T> {
	fn from(arr: [T; 5]) -> Self {
		MyVec(arr.to_vec())
	}
}
impl<T: Clone> From<MyVec<T>> for Vec<T> {
	fn from(arr: MyVec<T>) -> Self {
		arr.0 // first element of the struct
		 // arr.to_vec()
	}
}

// Overload via traits:
pub trait Adds<T> {
	fn add(self, rhs: T) -> Self;
}

impl Adds<i32> for i32 {
	fn add(self, rhs: i32) -> Self {
		self + rhs
	}
}

impl Adds<&str> for String {
	fn add(mut self, rhs: &str) -> Self {
		self.push_str(rhs);
		self
	}
}


impl Adds<&str> for Vec<String> {
	fn add(mut self, rhs: &str) -> Self {
		self.push(rhs.to_string());
		self
	}
}


impl Adds<i32> for Vec<i32> {
	fn add(mut self, rhs: i32) -> Self {
		self.push(rhs);
		self
	}
}

pub fn map<T, U, F>(items: Vec<T>, func: F) -> Vec<U>
where
	F: FnMut(T) -> U,
{
	items.into_iter().map(func).collect()
}
