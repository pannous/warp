//! card rust-paths: Rust's paths `A::b` read as warp (decision rust-forms-welcome): the standard library's constructors
//! as warp's literals, `std::…::word` and `f64::word` as the word, `i32::MAX` as its number, a turbofish `::<T>`
//! dropped, `.iter()` and `.collect()` the list itself
use crate::is;

#[test]
fn a_turbofish_says_nothing_warp_needs() {
	is!("[1, 2, 3].iter().sum::<i32>()", 6);
	is!("[1, 2, 3].iter().map(|x| x * 2).collect::<Vec<i32>>() == [2, 4, 6]", true);
	is!("let v: Vec<i32> = [1, 2].into_iter().collect(); v.len()", 2);
}

#[test]
fn standard_constructors_are_literals() {
	is!("String::from(\"hi\")", "hi");
	is!("let mut s = String::new(); s += \"a\"; s", "a");
	is!("let mut v = Vec::new(); v.push(3); v.len()", 1);
	is!("let mut m = HashMap::new(); m[\"a\"] = 1; m[\"a\"]", 1);
	is!("use std::collections::HashMap; let mut m = HashMap::new(); m[\"x\"] = 2; m[\"x\"]", 2);
}

#[test]
fn standard_paths_are_the_word() {
	is!("std::cmp::max(2, 3)", 3);
	is!("f64::sqrt(16.0)", 4.0);
	is!("std::f64::consts::PI == pi", true);
	is!("i32::MAX", 2147483647);
	is!("u8::MAX", 255);
	is!("i64::MIN < 0", true);
}

#[test]
fn to_string_is_string() {
	is!("3.to_string()", "3");
}

#[test]
fn an_enum_path_stays_the_case() {
	is!("enum C { Red, Green }; let c = C::Green; match c { C::Red => 1, C::Green => 2 }", 2);
}
