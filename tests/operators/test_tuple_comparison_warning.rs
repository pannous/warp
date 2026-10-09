// `(a, b) == x, y` compares the tuple with x only (the comma binds looser than ==): a strong warning suggests
// parentheses (user, P42); since P227 (user, 2026-10-09) it is an error asking for them
use warp::diagnostic::take_warnings;
use crate::is;

fn warns_about_parentheses(code: &str) -> bool {
	take_warnings();
	warp::wasm_emitter::eval(code).serialize().contains("binds looser than ==: write")
}

#[test]
fn a_tuple_compared_with_a_bare_comma_list_warns() {
	assert!(warns_about_parentheses("(2 as float, 4.3 as int) == 2.0, 4"));
	assert!(warns_about_parentheses("x = (1, 2) == 1, 2; x"));
}

#[test]
fn a_parenthesized_tuple_comparison_does_not_warn() {
	take_warnings();
	is!("(2 as float, 4.3 as int) == (2.0, 4)", 1);
	assert!(take_warnings().iter().all(|warning| !warning.message.contains("binds looser")));
}
