//! Card bool-conversion (found by the type model, KNOWN_VALUE_DIFFERENCES): a conversion to bool is a bool, printing
//! yes/no (notes/bool_type.md), not the int 1/0
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_conversion_to_bool_is_a_bool() {
	assert_eq!(text_of("2 as bool"), "yes");
	assert_eq!(text_of("0 as bool"), "no");
	assert_eq!(text_of("bool(2)"), "yes");
	assert_eq!(text_of("x = 2 as bool; x"), "yes");
	assert_eq!(text_of("n = 3; n as bool"), "yes");
	assert_eq!(text_of("'no' as bool"), "no");
}

#[test]
fn a_function_declared_bool_returns_a_bool() {
	assert_eq!(text_of("def f(x) -> bool { x }; f(2)"), "yes");
	assert_eq!(text_of("def f(x) -> bool { x }; f(0)"), "no");
}
