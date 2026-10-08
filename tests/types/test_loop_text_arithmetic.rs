//! Text arithmetic in a loop body is checked as outside a loop: text + anything is a text (card loop-text, the type
//! model in tests/types/test_type_model.rs)
use crate::common::fails_with;
use crate::is;

#[test]
fn text_arithmetic_in_a_loop_body_is_no_type_error() {
	is!("for c in [\"a\"] { print(c + 1) }; 0", 0);
	is!("n = 0; for c in [\"a\"] { c + 1; n++ }; n", 1);
	is!("n = 0; for c in \"ab\" { c + \"!\"; n++ }; n", 2);
	is!("s = \"x\"; for c in \"ab\" { s = c + 1 }; s", "b1");
	is!("t = \"\"; for c in \"ab\" { t = c + \"!\" }; t", "b!");
}

#[test]
fn list_arithmetic_in_a_loop_body_stays_a_type_error() {
	fails_with("for x in [[1]] { x + 2 }", "list + int");
}
