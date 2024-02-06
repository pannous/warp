use std::fmt::Debug; // for println!("{:?}", item)
use std::fmt::Display; // for println!("{}", item)
use crate::put;

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

pub(crate) trait CharExtensions {
    fn upper(&self) -> char;
}
impl CharExtensions for char {
    fn upper(&self) -> char {
        self.to_uppercase().next().unwrap()
    }
}

pub(crate) trait StringExtensions {
    fn upper(&self) -> String;
    fn reverse(&self) -> String;
    fn map(&self, f: fn(char) -> char) -> String;
    fn substring(&self, start: usize, end: usize) -> &str;
}

impl StringExtensions for str {
    fn upper(&self) -> String {
        self.to_uppercase()
    }
    fn reverse(&self) -> String {
        self.chars().rev().collect()
    }

    fn map(&self, f: fn(char) -> char) -> String {
        self.chars().map(f).collect()
    }
    fn substring(&self, start: usize, end: usize) -> &str {
        // just use the range operator directly
        &self[start..end]
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
        put!(item);
    }
}

// by reference
// fn print_list_<'a, T: std::fmt::Display + 'a>(list: impl IntoIterator<Item = &'a T> /*(*/ ) {
//     for item in list {
//         put!(item);
//     }
// }

// fn printList<T>(list: Vec<T>) {
//     for item in list {
//         println!("$item ")
//     }
// }
// fn printList<T>(list: [T;N]) {
//     printList(list.to_vec());
// }
