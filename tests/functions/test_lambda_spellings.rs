// Lambdas as other languages write them: Rust `|x| x*2`, Ruby `{ |x| x*x }`, Kotlin `{ x -> x*2 }`, Rust/Go-like
// `fn(a, b) { a*b }`, Python `lambda x: x*2`, JS `function(a, b) { … }`
use crate::is;
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn pipe_parameters() {
	is!("f = |x| x*2; f(3)", 6); // Rust
	is!("f = |a, b| a*b; f(2,3)", 6);
	is!("f = |a b| a+b; f(2,3)", 5);
	assert_eq!(printed("map([1,2,3], |x| x*x)"), "[1 4 9]");
	assert_eq!(printed("[1,2,3,4].filter(|x| x % 2 == 0)"), "[2 4]");
}

#[test]
fn ruby_block_parameters() {
	assert_eq!(printed("map([1,2,3], { |x| x*x })"), "[1 4 9]");
	assert_eq!(printed("[1,2,3].map { |x| x+1 }"), "[2 3 4]");
	is!("fold([1,2,3], 0, { |acc, x| acc + x })", 6);
}

#[test]
fn other_lambda_keywords() {
	is!("f = fn(a, b) { a*b }; f(2,3)", 6);
	is!("f = lambda x: x*2; f(4)", 8);
	is!("f = function(a, b) { return a*b }; f(2,3)", 6);
	is!("f = { x -> x*2 }; f(3)", 6); // Kotlin
	assert_eq!(printed("map([1,2,3], { x -> x*10 })"), "[10 20 30]");
}
