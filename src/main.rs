#![allow(dead_code, unused_imports)]

mod extensions;

// use crate::extensions::*; // crate for F12
use extensions::lists::*;
use extensions::numbers::*;
use extensions::strings::*;
use extensions::utils::*;

pub mod node;
pub mod wasm_gc_emitter;
pub mod wasm_gc_reader;
pub mod wasp_parser;

pub mod type_kinds;

pub mod ast;
pub mod meta;
// ⚠️ modules also need to be used in main.rs AND lib.rs to be compiled

// use bla::test_bla_lib;

// glob import doesn't reexport anything because no candidate is public enough
// reexporting is done by pub use
// pub mod extensions;
// pub use extensions::*;

// typedef Vec<String> StringVec in rust:
type Strings = Vec<String>;

#[allow(unused_variables)] // for testing
#[cfg(not(any(feature = "wasm", test)))]
fn main() {
	println!("Warp 🎡𐃏 ☸ WASM Building Program");
	// test_bla_lib();
	let url = "https://files.pannous.com/test";
	let test: String = download(url);
	let n = Number::Int(5);
	let f = Number::Float(5.0);
	let c = Number::Complex(5.0, 7.0);
	let q = Number::Quotient(5, 7);
	let f: f64 = (q / n).into();
	// put!((q/n) as f64);
	// put!("q.sign();
	put!("test ", test);

	let ranges = 1..10;
	let range: Vec<i32> = ranges.collect();
	// let range1:Vec<i32> = ranges.into();
	put!("range ", range);

	// tests are in a separate module, usually not accessed from the main module
	// string_tests::test_all();

	let upper = "hello 🌍".map(|c| c.upper());
	put!("upper reverse ", upper.reverse());

	// list of 5 numbers
	let numbers = [1, 2, 3, 4, 5];
	let xxs: List<Number> = numbers.into();
	// map each number to a string
	let numbers_as_chars = numbers.map(|n| n.to_char());
	print_list(numbers_as_chars);
	// let numbers_as_strings = numbers.map(|n| n.to_string());
}

#[cfg(test)]
fn main() {
	print!("test");
}
