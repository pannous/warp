//! Functions that compute texts: text-valued `if`, `return` of a text or error from a statement, `error(message)`

use warp::error;
use crate::is;

#[test]
fn an_if_with_text_branches_makes_a_text_function() {
	is!("f(x):= if (x > 1) {\"big\"} else {\"small\"}; f(3)", "big");
	is!("f(x):= if (x > 1) {\"big\"} else {\"small\"}; f(1)", "small");
	is!("f(x):= if (x > 1) {byte_slice(\"abc\",0,x)} else {\"small\"}; f(3)", "abc");
}

#[test]
fn return_from_a_statement_gives_back_the_node() {
	is!("f(x):={ if x > 1 { return \"b\" + \"c\" }; \"small\" }; f(3)", "bc");
	is!("f(x):={ if x > 1 { return error(\"big\") }; \"small\" }; f(2)", error("big"));
	is!("f(x):={ if x > 1 { return 5 }; 7 }; f(3)", 5);
}

#[test]
fn a_block_with_an_if_statement_is_code_not_data() {
	is!("f(x):={ if x > 1 {3}; \"small\" }; f(0)", "small");
	is!("{ if 1 {3}; 4 }", 4);
}

#[test]
fn error_makes_an_error_value_of_a_text() {
	is!("error(\"x\")", error("x"));
	is!("error(\"unknown: \" + \"beta\")", error("unknown: beta"));
}
