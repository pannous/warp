// card leading-minus: `-floor 2.5` worked as a statement, but `1 + -floor 2.5` said "undefined variable: floor": the
// word after a prefix minus takes its braceless argument as it does after `+` (warp_parser parse_prefix_operand)
use crate::is;

#[test]
fn a_negated_call_negates_the_call_wherever_it_stands() {
	is!("1 + -floor 2.5", -1);
	is!("1 - -floor 2.5", 3);
	is!("1 + -sin 0", 1);
	is!("f(x):=x; 1 + -f 3", -2);
	is!("x = -floor 2.5; x", -2);
	is!("def f():=1 + -floor 2.5; f()", -1);
	is!("if 1 + -floor 2.5 < 0 { 7 } else { 8 }", 7);
	is!("xs=[1 2]; -count xs", -2);
}

#[test]
fn a_negated_variable_takes_no_argument() {
	is!("x=2; [-x 1]", warp::ints(vec![-2, 1]));
	is!("x=5; x -1", 4);
}
