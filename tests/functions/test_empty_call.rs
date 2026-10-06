// `foo()` with explicit empty parentheses is a call (user, P92, wiki/charged.md §1): an undefined name is
// "undefined function: foo", never the symbol foo; a bare `foo` and the group `(foo)` stay the symbol
use crate::is;

#[test]
fn an_empty_call_of_an_undefined_name_is_an_error() {
	crate::common::fails_with("foo()", "undefined function: foo");
	crate::common::fails_with("1; foo()", "undefined function: foo");
	crate::common::fails_with("x = foo(); 1", "undefined function: foo");
}

#[test]
fn a_name_and_a_group_stay_symbols_and_defined_calls_run() {
	is!("foo", warp::Node::Symbol("foo".to_string()));
	is!("(foo)", warp::Node::Symbol("foo".to_string()));
	is!("f():=3; f()", 3);
	is!("xs=[1,2]; xs.count()", 2);
	is!("random() < 1", true);
}
