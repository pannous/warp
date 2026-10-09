// cards golf-text, golf-sum: `or` / `||` between computed text and a number falls back to the number when the text is
// empty (notes/code_golf.md, fizz-buzz)
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn repeated_text_or_a_number() {
	assert_eq!(shown("x = 2; \"a\"*(x<2) or x"), "2");
	assert_eq!(shown("x = 1; \"a\"*(x<2) or x"), "\"a\"");
	assert_eq!(shown("x = 2; \"a\"*(x<2) || x"), "2");
}

#[test]
fn a_sum_of_repeated_texts_or_a_number() {
	assert_eq!(shown("x = 7; \"Fizz\"*(x%3<1)+\"Buzz\"*(x%5<1) || x"), "7");
	assert_eq!(shown("x = 15; \"Fizz\"*(x%3<1)+\"Buzz\"*(x%5<1) || x"), "\"FizzBuzz\"");
	assert_eq!(shown("x = 5; (\"Fizz\"*(x%3<1)+\"Buzz\"*(x%5<1)) or x"), "\"Buzz\"");
}
