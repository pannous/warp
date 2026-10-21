// card rust-lane-type-2: `if b < 0 then -b else b` of a parameter b read `-b else` as the call b(else), so float_mod of
// lib/compiler/runtime/conversions.warp failed "b is a float where an exact Int is expected"; an operator word after a
// negated name is the operator, as after any operand
use crate::is;

#[test]
fn an_operator_word_after_a_negated_name_is_the_operator() {
	is!("f(b: float): float := if b < 0 then -b else b\nf(-2.5)", 2.5);
	is!("f(b: float): float := if b < 0 then -b else b\nf(1.5)", 1.5);
	is!("f(x: int): int := -x or 2\nf(0)", 2);
	is!("f(x: int, y: int) := -x and y\nf(1, 0)", 0);
}
