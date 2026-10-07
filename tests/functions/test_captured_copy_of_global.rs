//! card captured-copy: a lambda capturing a function local that copies a main-level variable (under = paper) gets the
//! local's kind from main's variable (it was an internal WASM validation error, or a null reference with an if)
use crate::is;

#[test]
fn a_lambda_captures_a_local_copied_from_main() {
	is!("paper = 7; def f(below) { under = paper; mix = (s) => under + s; mix(1) }; f(0)", 8);
	is!("paper = 7; def f(below) { under = if below == 0 then paper else below; mix = (s) => under + s; mix(1) }; f(0)", 8);
	is!("paper = 7; def f(below) { under = if below == 0 then paper else below; mix = (s) => under + s; mix(1) }; f(2)", 3);
}
