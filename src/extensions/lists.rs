// LIST AND VECTOR EXTENSIONS

use crate::extensions::numbers::Number;
use crate::node::Node;
use std::array::IntoIter;
use std::iter::Enumerate;

use std::ops::Range;
use wasmparser::{FromReader, SectionLimited};

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

//
// impl<T> Indexed<T> for SectionLimited<'_, T> {
//     fn indexed(self)-> impl Iterator<Item = (usize, T)>{
//         self.into_iter().enumerate()
//     }
// }

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
// impl<T> FromRange<T> for Vec<T> {
//     fn from(range: Range<T>) -> Self {
//         range.collect()
//     }
// }

// use std::Vec;
// | |                `Vec` is not defined in the current crate
// | impl doesn't use only types from inside the current crate
// {} is for strings and other values which can be displayed directly to the user.
// There's no single way to show a vector to a user.
// The {:?} formatter can be used to debug it, and it will look like:
// println!("{:?}", vec![1; 10]); // OK
// impl Display for Vec<i32> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "[")?;
//         for (i, item) in self.iter().enumerate() {
//             if i > 0 {
//                 write!(f, ", ")?;
//             }
//             write!(f, "{}", item)?;
//         }
//         write!(f, "]")
//     }
// }

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

// use paste::paste; // paste! macro to generate code for general case [i32; N]
// macro_rules! impl_from_array {
//     ($N:expr) => {
//         paste! {
//             impl From<[i32; $N]> for extensions::List::List<extensions::Numbers::Number> {
//                 fn from(array: [i32; $N]) -> Self {
//                     let data = array.iter().map(|&x| extensions::Numbers::Number(x)).collect();
//                     extensions::List::List { data }
//                 }
//             }
//         }
//     };
// }

// Generate From implementations for different array lengths
// impl_from_array!(1);
// impl_from_array!(2);
// impl_from_array!(3);
// impl_from_array!(4);
// impl_from_array!(5);
// impl_from_array!(6);
// impl_from_array!(7);
// impl_from_array!(8);
// impl_from_array!(9);

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
pub(crate) trait VecExtensions<T: Clone> {
	fn with(&self, n: T) -> Vec<T>;
	// fn filter(&self, f: &dyn Fn(&T) -> bool) -> Vec<T>;
	// fn find(&self, f: &dyn Fn(&T) -> bool) -> Option<&T>;
	fn filter(&self, f: fn(&T) -> bool) -> Vec<T>;
	fn find(&self, f: fn(&T) -> bool) -> Option<&T>;
}

impl<T: Clone> VecExtensions<T> for Vec<T> {
	fn with(&self, n: T) -> Vec<T> {
		let mut v = self.clone();
		v.push(n);
		v
	}
	fn filter(&self, f: fn(&T) -> bool) -> Vec<T> {
		self.iter().filter(|item| f(item)).cloned().collect()
	}
	fn find(&self, f: fn(&T) -> bool) -> Option<&T> {
		self.iter().find(|item| f(item))
	}
}

pub(crate) trait VecExtensions2<T: Clone> {
	fn with(&self, n: T) -> Vec<T>;
	fn filter(&self, f: &dyn Fn(&T) -> bool) -> Vec<T>;
	fn find2(&self, f: &dyn Fn(&T) -> bool) -> Option<&T>;
}

impl<T: Clone> VecExtensions2<T> for Vec<T> {
	fn with(&self, n: T) -> Vec<T> {
		let mut new_vec = self.clone();
		new_vec.push(n);
		new_vec
	}

	fn filter(&self, f: &dyn Fn(&T) -> bool) -> Vec<T> {
		self.iter().filter(|&x| f(x)).cloned().collect()
	}

	fn find2(&self, f: &dyn Fn(&T) -> bool) -> Option<&T> {
		self.iter().find(|&x| f(x))
	}
}
pub(crate) trait ArrayExtensions<T> {
	fn push(&self, n: T) -> Vec<T>;
}

// ⚠️ Rust does not currently support generic traits over arrays with a dynamic length. Use Vec<T>
// impl<T> ArrayExtensions<T> for [T; N]  ?
impl<T: Clone> ArrayExtensions<T> for [T; 5] {
	fn push(&self, n: T) -> Vec<T> {
		let mut v = self.to_vec();
		v.push(n);
		v
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
// impl<T> From<[T; 5]> for Vec<T> {
//     fn from(arr: [T; 5]) -> Self {
//         arr.to_vec()
//     }
// }
//
// fn push(&self, n: i32) -> Vec<i32> {
//     let mut v = *self.to_vec();
//     v.push(n);
//     v
// }

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

// impl Add<&str> for Strings {
//     fn add(mut self, rhs: &str) -> Self {
//         self.push(rhs.to_string());
//         self
//     }
// }

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
