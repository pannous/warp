//! true/false are a shallow type of their own (card bool-type, notes/decisions.md): they act as 1/0 in arithmetic and
//! comparisons, but `type(true)` is bool and they print as true/false; `===` also compares the type (card zero-false)
use warp::wasm_emitter::eval;
use crate::is;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_comparisons_print_as_bool() {
	assert_eq!(text_of("3 > 2"), "true");
	assert_eq!(text_of("1 > 2"), "false");
	assert_eq!(text_of("x = 3 > 2; x"), "true");
	assert_eq!(text_of("not 3"), "false");
	assert_eq!(text_of("f(x) := x > 2; f(3)"), "true");
}

#[test]
fn test_bool_is_a_type_of_its_own() {
	is!("type(true)", "bool");
	is!("type(3 > 2)", "bool");
	is!("true is bool", true);
	is!("1 is bool", false);
}

#[test]
fn test_bool_acts_as_one_or_zero() {
	is!("x = 3 > 2; x + 1", 2);
	is!("0 == false", true);
	is!("1 == true", true);
}

#[test]
fn test_strict_equality_compares_the_type() {
	assert_eq!(text_of("0 === false"), "false");
	assert_eq!(text_of("1 === true"), "false");
	assert_eq!(text_of("1 === 1"), "true");
	assert_eq!(text_of("true === true"), "true");
	assert_eq!(text_of("(3 > 2) === true"), "true");
	assert_eq!(text_of("0 !== false"), "true");
	assert_eq!(text_of("1 !== 1"), "false");
}

#[test]
fn a_bool_of_unknown_static_type_is_tested_at_run_time() {
	is!("h(value:any) := value is int; h(false)", false);
	is!("h(value:any) := value is bool; h(false)", true);
	is!("h(value:any) := value is int; h(3)", true);
	is!("h(value:any) := value == 0; h(false)", true);
	is!("h(value:any) := value == true; h(1)", true);
}
