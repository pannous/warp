// card leading-minus: `-floor 2.5` worked as a statement, but after an operator `1 + -floor 2.5` said "undefined
// variable: floor": a negated braceless call is `-(w x)` as the right operand of any operator or assignment
use crate::is;

#[test]
fn a_negated_call_after_an_operator_negates_the_call() {
	is!("1 + -floor 2.5", -1);
	is!("1 - -floor 2.5", 3);
	is!("1 + -sin 0", 1);
	is!("f(x):=x; 1 + -f 3", -2);
	is!("x = -floor 2.5; x", -2);
}
