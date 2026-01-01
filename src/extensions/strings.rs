use std::fmt::Debug; // for println!("{:?}", item)
use std::fmt::Display; // for println!("{}", item)
// use crate::put;

struct WasmString {
    length: u32,
    data: *const u8,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
pub fn String(s: &str) -> String {
    // ⚠️ pseudo constructor no conflict with std::string::String ??
    s.to_string()
}

#[allow(dead_code)]
#[allow(non_snake_case)]
pub fn S(s: &str) -> String {
    // String.from(s)
    s.to_string()
}

// byte literal b'A' b"hello" &[u8]
pub trait CharExtensions {
    fn upper(&self) -> char;
    fn s(&self) -> String;
}
impl CharExtensions for char {
    fn upper(&self) -> char {
        self.to_uppercase().next().unwrap()
    }
    fn s(&self) -> String { self.to_string() }
}

pub trait StringExtensions {
    fn at(&self,nr:i32) -> char;
    fn codepoint_at(&self, nr:i32) -> char;
    fn char(&self,nr:usize) -> char;
    fn upper(&self) -> String;
    fn reverse(&self) -> String;
    fn map(&self, f: fn(char) -> char) -> String;
    fn substring(&self, start: usize, end: usize) -> &str;
    fn first_char(&self) -> char;
    fn last_char(&self) -> char;
    fn first(&self) -> char;
    fn start(&self) -> char;
    fn head(&self) -> char;
    fn byte_at(&self, nr: usize) -> u8;
    fn str(&self) -> String;
    // allow negative index into chars : -2 = next to last
    fn s(&self) -> String;
    // from is reserved for String.from("…") constructor
    // fn from(&self, start: usize) -> &str;
    fn start_from(&self, start: usize) -> &str;
    fn set(&self, index: usize, c: char) -> String;
    // return the slice after the first occurrence of `pat`, or empty slice if not found
    fn after(&self, pat: &str) -> &str;
}


impl StringExtensions for String {
    // Function implementations
    fn s(&self) -> String { self.to_owned() }
    fn str(&self) -> String { self.to_owned() }
    fn at(&self, nr: i32) -> char {
        let wrapped_index = (nr + self.chars().count() as i32) as usize % self.chars().count();
        self.chars().nth(wrapped_index).unwrap()
    }
    fn codepoint_at(&self, nr:i32) -> char {self.at(nr)}
    fn char(&self, nr: usize) -> char { self.chars().nth(nr).unwrap()}
    fn byte_at(&self, nr: usize) -> u8 { self.as_bytes()[nr]}
    fn last_char(&self) -> char { self.chars().last().unwrap() }
    fn upper(&self) -> String {
        self.to_uppercase()
    }
    fn reverse(&self) -> String {
        self.chars().rev().collect()
    }
    fn first_char(&self) -> char { self.chars().next().unwrap() }
    fn first(&self) -> char { self.chars().next().unwrap() }
    fn start(&self) -> char { self.chars().next().unwrap() }
    fn head(&self) -> char { self.chars().next().unwrap() }
    fn map(&self, f: fn(char) -> char) -> String {
        self.chars().map(f).collect()
    }
    fn substring(&self, start: usize, end: usize) -> &str {
        // just use the range operator directly
        &self[start..end]
    }
    fn after(&self, pat: &str) -> &str {
        match self.find(pat) {
            Some(idx) => &self[idx + pat.len()..],
            None => &self[self.len()..],
        }
    }
    fn set(&self, at: usize, value: char) -> String { self.clone().replace_range(at..at+1, &value.to_string()); self.to_string() }
    // fn from(&self, start: usize) -> &str { &self[start..] }
    fn start_from(&self, start: usize) -> &str { &self[start..] }
    // fn start_from(&self, start: usize) -> &str { panic!("just use &s[start..] ") }
}

impl StringExtensions for str {
    // allow negative index into chars : -2 = next to last
    fn s(&self) -> String { self.to_string() }
    fn str(&self) -> String { self.to_string() }
    fn at(&self,nr:i32) -> char {
        let wrapped_index = (nr + self.chars().count() as i32) as usize % self.chars().count();
        self.chars().nth( wrapped_index).unwrap()
    }
    fn codepoint_at(&self, nr:i32) -> char {self.at(nr)}
    fn char(&self, nr: usize) -> char { self.chars().nth(nr).unwrap()}
    fn byte_at(&self, nr: usize) -> u8 { self.as_bytes()[nr]}
    fn last_char(&self) -> char { self.chars().last().unwrap() }
    fn upper(&self) -> String {
        self.to_uppercase()
    }
    fn reverse(&self) -> String {
        self.chars().rev().collect()
    }
    fn first_char(&self) -> char { self.chars().next().unwrap() }
    fn first(&self) -> char { self.chars().next().unwrap() }
    fn start(&self) -> char { self.chars().next().unwrap() }
    fn head(&self) -> char { self.chars().next().unwrap() }
    fn map(&self, f: fn(char) -> char) -> String {
        self.chars().map(f).collect()
    }
    fn after(&self, pat: &str) -> &str {
        match self.find(pat) {
            Some(idx) => &self[idx + pat.len()..],
            None => &self[self.len()..],
        }
    }
    fn start_from(&self, start: usize) -> &str { &self[start..] }
    // fn start_from(&self, start: usize) -> &str { panic!("just use &s[start..] ") }

    fn substring(&self, start: usize, end: usize) -> &str {
        // just use the range operator directly
        &self[start..end]
    }

    fn set(&self, at: usize, value: char) -> String {
        let mut changed=self.to_string();
        changed.replace_range(at..at+1, &value.to_string());
        changed
    }
}

pub(crate) trait IntegerExtensions {
    fn to_char(&self) -> char; // 3 -> '3' … 10 -> panic
}

impl IntegerExtensions for i32 {
    fn to_char(&self) -> char {
        match *self {
            0..=9 => std::char::from_digit(*self as u32, 10).unwrap(),
            10 => 'A',
            11 => 'B',
            12 => 'C',
            13 => 'D',
            14 => 'E',
            15 => 'F',
            100 => '💯',
            1000 => '𓆼', // 𐄢
            10000 => '𓂭', // 𐄫
            100000 => '𓆐',
            1000000 => '𓁨',
            _ => '…',
        }
    }

}



// by value
// call with &arg if you encounter "Borrow of moved value" error (later)
pub fn print_list<T: Display + Debug>(list: impl IntoIterator<Item = T>) {
    for item in list {
        println!("{}",item);
    }
}


use std::cmp::PartialEq;
// only traits defined in the current crate can be implemented for types defined outside of the crate
// use Wrapper or compare via s == *s2
// impl PartialEq for String {
//     fn eq(&self, other: &Self) -> bool {
//         &self.0 == &other.0
//     }
// }

// only traits defined in the current crate can be implemented for types defined outside of the crate
// impl PartialEq<str> for char {
//     fn eq(&self, other: &str) -> bool {
//         other.len() == 1 && other.chars().next() == Some(*self)
//     }
// }
//
// impl PartialEq<char> for str {
//     fn eq(&self, other: &char) -> bool {
//         self.len() == 1 && self.chars().next() == Some(*other)
//     }
// }

// use std::cmp::PartialEq;

trait PartialEqStr {
    fn is(&self, other: &str) -> bool;
}
trait PartialEqChar {
    fn is(&self, other: &char) -> bool;
}

trait PartialEqNum {
    fn is(&self, other: &i64) -> bool;
}

impl PartialEqStr for char {
    fn is(&self, other: &str) -> bool {
        other.len() == 1 && other.chars().next() == Some(*self)
    }
}

// impl PartialEqNum for char {
//     fn is(&self, other: &i64) -> bool {
//         self.to_digit(10) == Some(*other as u32)
//     }
// }

impl PartialEqChar for str {
    fn is(&self, other: &char) -> bool {
        self.len() == 1 && self.chars().next() == Some(*other)
    }
}

impl PartialEqChar for String {
    fn is(&self, other: &char) -> bool {
        self.as_str() == other.to_string()
    }
}


// fn assert<T: PartialEq + Debug>(x: T) {
//     assert_eq!(x, true);
// }
// fn assert(x: bool) {
//     assert_eq!(x, true);
// }
// macro_rules! assert {
//     ($x:expr) => {
//         assert_eq!($x, true);
//     };
// }

#[cfg(test)]
mod tests {
    use crate::eq;
    use super::*;

    #[test]
    fn test_char_str_eq() {
        assert!('l'.is("l"));
        // assert!('1'.is(1));
        // assert!("l".is('l'));
    }


    #[test]
    fn test_str_plus() {
        // eq!("a"+"b", "ab");
        eq!("a".s()+"b", "ab");
        // eq!("a".s()+2, "a2");
    }

}

#[allow(unused)]
macro_rules! s {
    ($lit:literal) => {
        String::from($lit)
    };
}

// Test it see tests/string_tests.rs !!
fn main() {
    let s1 = String::from("RustRover");
    let s2 = &String::from("RustRover");

    assert_eq!(s1 == *s2, true);
}
