//! `if f(1, 2) {…}`: the call's arguments stay the arguments, the block is the body

use warp::{is, parse};

#[test]
fn a_call_with_arguments_as_condition_keeps_its_arguments() {
	let both = "both(a, b) := a > 0 and b > 0; ok = 0;";
	is!(&format!("{both} if both(1, 2) {{ ok = 1 }}; ok"), 1);
	is!(&format!("{both} next = 2; if both(1, next) {{ ok = 1 }}; ok"), 1);
	is!(&format!("{both} if both(1, -2) {{ ok = 1 }} else {{ ok = 2 }}; ok"), 2);
	is!(&format!("{both} if not both(1, -2) {{ ok = 1 }}; ok"), 1);
	is!(&format!("{both} i = 0; while both(1, 3 - i) {{ i++ }}; i"), 3);
}

#[test]
fn a_call_condition_parses_like_the_assigned_call() {
	assert_eq!(parse("if f(1, 2) {ok=1}").to_string(), parse("if f(1, 2) then {ok=1}").to_string());
}
