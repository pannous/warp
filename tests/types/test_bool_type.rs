//! true/false are a shallow type of their own (card bool-type, notes/decisions.md): they act as 1/0 in arithmetic and
//! comparisons, but `type(true)` is bool and they print as true/false; `===` also compares the type (card zero-false)
use warp::wasm_emitter::eval;
use crate::is;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_comparisons_print_as_bool() {
	assert_eq!(text_of("3 > 2"), "yes");
	assert_eq!(text_of("1 > 2"), "no");
	assert_eq!(text_of("x = 3 > 2; x"), "yes");
	assert_eq!(text_of("not 3"), "no");
	assert_eq!(text_of("f(x) := x > 2; f(3)"), "yes");
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
	assert_eq!(text_of("0 === false"), "no");
	assert_eq!(text_of("1 === true"), "no");
	assert_eq!(text_of("1 === 1"), "yes");
	assert_eq!(text_of("true === true"), "yes");
	assert_eq!(text_of("(3 > 2) === true"), "yes");
	assert_eq!(text_of("0 !== false"), "yes");
	assert_eq!(text_of("1 !== 1"), "no");
}

#[test]
fn a_bool_of_unknown_static_type_is_tested_at_run_time() {
	is!("h(value:any) := value is int; h(false)", false);
	is!("h(value:any) := value is bool; h(false)", true);
	is!("h(value:any) := value is int; h(3)", true);
	is!("h(value:any) := value == 0; h(false)", true);
	is!("h(value:any) := value == true; h(1)", true);
}

#[test]
fn a_literal_bool_counts_in_arithmetic() {
	is!("true + 1", 2); // P195 (user)
	is!("true + true", 2);
}

#[test]
fn strict_equality_compares_the_full_type() {
	// P196 (user): same value and same type
	assert_eq!(text_of("1 === 1.5"), "no");
	assert_eq!(text_of("1 === (1.0 as float)"), "no");
	assert_eq!(text_of("\"a\" === \"a\""), "yes");
	assert_eq!(text_of("1 !== 1.5"), "yes");
}

#[test]
fn a_list_starting_with_a_bool_is_a_list() {
	// card true-lowers: `[true, 2]` was taken for a law (`law 2`) and lowered to ø
	assert_eq!(text_of("[true, 2]"), "[yes 2]");
	assert_eq!(text_of("[yes 2]"), "[yes 2]");
}

// card bool-loses: an untyped parameter given a bool at every call is one, as if written `b: bool`
#[test]
fn a_parameter_given_bools_keeps_them() {
	is!("show(b) := \"got \" + b; show(1<2)", "got yes"); // was "got 1"
	is!("show(b) := \"got \" + b; ok = 3 > 2; show(ok)", "got yes");
	assert_eq!(text_of("f(b) := b; f(1 == 1)"), "yes");
	is!("show(b) := \"got \" + b; show(1<2); show(5)", "got 5"); // given a number too: no bool
	is!("f(b) := b + 1; f(1 == 1)", 2);
}

// an exact comparison, evaluated at compile time, is a truth like any other; it printed as the Int 1
#[test]
fn an_exact_comparison_prints_as_bool() {
	assert_eq!(text_of("√2 > 1.4"), "yes");
	assert_eq!(text_of("π < 3"), "no");
}
