//! card sqrt-round: `sqrt(2).round(2)` applies round to sqrt(2), as `sin(2).round(2)` does; the operator word sqrt
//! glued to its parentheses takes only them as operand when a method follows
use crate::is;

#[test]
fn a_method_on_an_operator_words_call_applies_to_its_result() {
	is!("sqrt(2).round(2)", 1.41);
	is!("abs(-1.6).round(0) + cbrt(8).round(0)", 4);
	is!("sqrt(16) + 1", 5.0);
	is!("sqrt 16.0", 4.0);
}
