use crate::extensions::numbers::Number;

use std::ops::Range;

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
// {} is for strings and other values which can be displayed directly to the user. There's no single way to show a vector to a user.
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


pub enum List<T>{
    Vector(Vec<T>),
    Array([T; 5]), // makes no sense to have a list of fixed array length
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


pub trait VecExtensions<T> {
    fn with(&self, n: T) -> Vec<T>;
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

impl<T: Clone> VecExtensions<T> for Vec<T> {
    fn with(&self, n: T) -> Vec<T> {
        let mut v = self.clone();
        v.push(n);
        v
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



// The "From" trait is used when you want to define your own custom conversion from one type to another.
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