// INCLUDE: ln ~/dev/script/rust/extensions.rs
// mod extensions; // also exports the macros declared #[macro_export]
// use crate::extensions::*; // crate for F12
// use extensions::strings::*;
// use extensions::lists::*;
// use extensions::numbers::*;

use crate::node::Node;
use crate::wasp_parser::parse;

pub mod lists;
pub mod numbers;
pub mod reals; // warp only
pub mod strings;
pub mod utils;

// Re-export all traits for convenience: `use extensions::*;`
pub use lists::*;
pub use numbers::*;
pub use strings::*;
pub use utils::*;

// s!(x) is x.to_string()
// better use "wtf".s() from extensions::strings
#[macro_export]
macro_rules! s {
	($str:expr) => {
		$str.to_string()
	};
}

#[macro_export]
macro_rules! texts { // Texts // boxed list of Text nodes
	($($lit:literal),* $(,)?) => {
		Node::List(vec![$(Node::Text(($lit).to_string())),*], Bracket::None, Separator::Colon)
	};
}

#[macro_export]
macro_rules! symbols { // boxed list of Symbol nodes
	($($lit:literal),* $(,)?) => {
		Node::List(vec![$(Node::Symbol(($lit).to_string())),*],Bracket::None, Separator::Space)
	};
}


#[macro_export]
macro_rules! expression { // boxed list of Symbol nodes
	($($lit:literal),* $(,)?) => {
		Node::List(vec![$(Node::Symbol(($lit).to_string())),*],Bracket::None, Separator::Space)
	};
}

// #[macro_export]
// macro_rules! ints { // just use primitive integer vec!


#[macro_export]
macro_rules! ints { // List of Int nodes   vs Data(vec![1])!
	($($lit:literal),* $(,)?) => {
		Node::List(vec![$(int($lit)),*], Bracket::None, Separator::Space)
	};
}


// Modules can reside in a file with the same name as the module,
// or in a file named mod.rs inside a directory with the same name as the module.
// so we can ON DEMAND put extensions in a dir called extensions AND keep extensions.rs for some

// no longer needed
// #[macro_use]
// extern crate extensions;


#[macro_export]
macro_rules! exists {
	($a:expr) => {{
		assert!(($a) != false);
	}};
}


#[macro_export]
macro_rules! parses_to { // parser eq!
	// Evaluate string expressions like "3+3" and roundtrip through WASM
	($a:expr, $b:expr) => {{
		let result = parse($a);
		assert_eq!(result, $b);
	}};
}







// PUB : PUBLIC FUNCTIONS
// you need to explicitly mark each function with the pub keyword in the module definition.
// Rust does NOT provide a way to globally set visibility for all items within a module;
#[allow(dead_code)]
pub fn public_function() {
	println!("public function");
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
	use crate::analyzer::analyze;
	let parsed = parse(code);
	if let Node::Error(_) = &parsed {
		return; // Parse error - test passes
	}
	// Also check for analysis errors (type mismatches, etc.)
	match analyze(parsed) {
		Node::Error(_) => (), // Analysis error - test passes
		_ => panic!("Expected error, but code parsed and analyzed successfully."),
	}
}

