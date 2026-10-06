// Anonymous functions as other languages write them are lambdas: JS `function(a, b) { … }`, Rust/Swift-ish
// `fn(a, b) { … }`, Python `lambda x: …`, Java/Kotlin-ish `(a, b) -> …`
use crate::is;

#[test]
fn a_function_keyword_without_a_name_is_a_lambda() {
	is!("f = function(a, b) { a + b }; f(3, 4)", 7); // JS
	is!("mul = fn(a, b) { a * b }; mul(2, 3)", 6);
	is!("f = function(x) { return x * 2 }; f(4)", 8);
	is!("map([1,2,3], fn(x) { x * 10 })", warp::ints(vec![10, 20, 30]));
}

#[test]
fn python_lambda_is_a_lambda() {
	is!("f = lambda x: x * 2; f(4)", 8);
	is!("map([1,2,3], lambda x: x + 1)", warp::ints(vec![2, 3, 4]));
}

#[test]
fn an_arrow_lambda_works_too() {
	is!("f = (a, b) -> a + b; f(3, 4)", 7);
}
