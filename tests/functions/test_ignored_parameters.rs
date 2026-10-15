//! Card def-hi: `_` in a definition's head is an ignored parameter, never the placeholder of a partial application
use crate::is;

#[test]
fn ignored_parameters_still_consume_arguments() {
	is!("def constant(_):=\"hi\"; constant(1)", "hi");
	is!("def middle(_, value, _):=value; middle(1, 2, 3)", 2);
}

#[test]
fn an_ignored_parameter_without_a_keyword() {
	is!("constant(_):=\"hi\"; constant(1)", "hi");
	is!("fun seven(_){ 7 }; seven(0)", 7);
}

#[test]
fn a_placeholder_in_a_call_stays_a_partial_application() {
	is!("add(a,b):=a+b; increment=add(1,_); increment(2)", 3);
}
