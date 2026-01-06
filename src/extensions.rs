// INCLUDE: ln ~/dev/script/rust/extensions.rs
// mod extensions; // also exports the macros declared #[macro_export]
// use crate::extensions::*; // crate for F12
// use extensions::Numbers::*;
// use extensions::Strings::*;

use crate::node::Bracket;
use crate::node::Separator;
use crate::Number::Int;
use crate::node::Node;
use crate::wasp_parser::parse;

pub mod lists;
pub mod numbers;
pub mod strings; // ⚠️ reexport still needs explicit import:
pub mod utils;
// use extensions::Numbers::*;

// #[allow(dead_code)]
// mod extensions {}

// fucking s!("to_string")
// better use "wtf".s() from extensions::strings
#[macro_export]
macro_rules! s {
	($str:expr) => {
		$str.to_string()
	};
}

#[macro_export]
macro_rules! strings { // ever used?
	($($lit:literal),* $(,)?) => {
		vec![$(String::from($lit)),*]
	};
}

#[macro_export]
macro_rules! Strings { // Texts // boxed list of Text nodes (why? inefficient without benefit?)
	($($lit:literal),* $(,)?) => {
		Node::List(vec![$(Node::Text(String::from($lit))),*],,Bracket::None, Separator::Space)
	};
}


// #[macro_export]
// macro_rules! ints { // just use primitive integer vec!


#[macro_export]
macro_rules! ints { // List of Int nodes   vs Data(vec![1])!
	($($lit:literal),* $(,)?) => {
		Node::List(vec![$(int($lit)),*],Bracket::None, Separator::Space)
	};
}


// Modules can reside in a file with the same name as the module,
// or in a file named mod.rs inside a directory with the same name as the module.
// so we can ON DEMAND put extensions in a dir called extensions AND keep extensions.rs for some

// no longer needed
// #[macro_use]
// extern crate extensions;

#[macro_export]
macro_rules! eq {
	// Evaluate string expressions like "3+3"
	($a:expr, $b:expr) => {{
		assert_eq!($a, $b);
	}};
}

#[macro_export]
macro_rules! exists {
	($a:expr) => {{
		assert!(($a) != false);
	}};
}


#[macro_export]
macro_rules! peq { // parser eq!
	// Evaluate string expressions like "3+3" and roundtrip through WASM
	($a:expr, $b:expr) => {{
		let result = parse($a);
		assert_eq!(result, $b);
	}};
}

#[macro_export]
macro_rules! is {
	// Evaluate string expressions like "3+3" and roundtrip through WASM
	($a:expr, $b:expr) => {{
		let result = wasp::wasm_gc_emitter::eval($a);
		assert_eq!(result, $b);
	}};
}

#[macro_export]
macro_rules! wis {
	// Wisp roundtrip: parse wisp -> Node -> emit wisp -> parse again -> compare
	($input:expr) => {{
		let node = wasp::wisp_parser::parse_wisp($input);
		let emitted = wasp::wisp_parser::emit_wisp(&node);
		let reparsed = wasp::wisp_parser::parse_wisp(&emitted);
		assert_eq!(node, reparsed, "roundtrip failed:\n  input: {}\n  emitted: {}", $input, emitted);
		node
	}};
	// Wisp eval: parse wisp -> Node -> compare to expected
	($input:expr, $expected:expr) => {{
		let node = wasp::wisp_parser::parse_wisp($input);
		assert_eq!(node, $expected, "wisp parse mismatch for: {}", $input);
		node
	}};
}

#[macro_export]
macro_rules! skip {
	($($t:tt)*) => {};
}
// skip arbitrary token sequence YAY also use todo!("but only for strings") !

// #[macro_export]
// macro_rules! skip {
//     ($a:expr) => {{
//     }};
// }

#[macro_export]
macro_rules! check {
	($cond:expr) => {{
		assert!($cond);
	}};
}

#[macro_export]
macro_rules! put {
        // ($($arg:tt)*) => (println!($($arg)*));
    ($($arg:expr),*) => {{
        $(print!("{:?}", $arg);)*
        println!(); // New line at the end
    }};
}

// PUB : PUBLIC FUNCTIONS
// you need to explicitly mark each function with the pub keyword in the module definition.
// Rust does NOT provide a way to globally set visibility for all items within a module;
#[allow(dead_code)]
pub fn public_function() {
	put!("public function");
}

// https://doc.rust-lang.org/std/primitive.char.html
// rust playground:
// https://play.rust-lang.org/?version=stable&mode=debug

#[macro_export]
macro_rules! printf {
    // partial implementation of printf => println! wrapper with format specifiers
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        println!(
            $crate::printf_fmt!($fmt),
            $($arg),*
        )
    };
}

#[macro_export]
macro_rules! printf_fmt {
	("%s") => {
		"{}"
	};
	("%d") => {
		"{}"
	};
	("%i") => {
		"{}"
	};
	("%u") => {
		"{}"
	};
	("%f") => {
		"{}"
	};
	("%lf") => {
		"{}"
	};
	("%c") => {
		"{}"
	};
	("%p") => {
		"{:p}"
	};
	($other:literal) => {
		$other
	}; // passthrough
}

pub fn print(msg: &str) {
	println!("{}", msg);
}

pub fn prints(msg: String) {
	println!("{}", msg);
}

// use should_panic !
// e.g. #[should_panic(expected = "Expected error, but code parsed successfully.")]
pub fn assert_throws(code: &str) {
	match parse(code) {
		Node::Error(_) => (), // Test passes if an error is thrown
		_ => panic!("Expected error, but code parsed successfully."),
	}
}

pub fn todow(msg: &str) {
	println!("{}", msg);
}

#[allow(unused)]
macro_rules! s {
	($lit:literal) => {
		String::from($lit)
	};
}
