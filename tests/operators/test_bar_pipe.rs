// `x | f` is `f(x)` when f names a function (D6: `|` dispatches on its operands, wiki/pipe.md); between values it
// stays the logical or. It binds below a braceless call: `square 2 | root` is `root(square(2))`, `fetch url | trim`
// trims the fetched text
use warp::{ints, is};

#[test]
fn a_bar_before_a_function_pipes_the_value_into_it() {
	is!("\"ab\" | upper", "AB");
	is!("square(x):=x*x; 3 | square", 9);
	is!("[3 1 2] | sort | reverse", ints(vec![3, 2, 1]));
	is!("s = \" ab \" | trim; s", "ab");
}

#[test]
fn a_bar_after_a_braceless_call_pipes_its_result() {
	is!("square(x):=x*x; square 2 | square", 16);
	is!("upper \"ab\" | reverse", "BA");
}

#[test]
fn a_bar_between_values_stays_the_logical_or() {
	is!("x=0; y=7; x | y", 7);
	is!("(1==1) | (2==3)", true);
	is!("f(a, b) := a | b; f(0, 3)", 3);
}
