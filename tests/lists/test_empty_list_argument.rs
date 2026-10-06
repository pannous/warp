//! `f([])` passes the empty list: an argument that is ø is still an argument

use warp::parse;
use crate::is;

#[test]
fn an_empty_list_literal_is_a_call_argument() {
	assert_eq!(parse("f([])").to_string(), "(f ø)");
	assert_eq!(parse("f(1,[])").to_string(), "(f 1 ø)");
	assert_eq!(parse("f([],1)").to_string(), "(f ø 1)");
}

#[test]
fn a_call_without_arguments_has_none() {
	assert_eq!(parse("f()").to_string(), "(f)");
}

#[test]
fn a_function_receives_the_empty_list() {
	is!("f(x):=count(x); f([])", 0);
	is!("both(a,b):=count(a)+count(b); both([],[7 8])", 2);
	is!("both(a,b):=count(a)+count(b); both([7 8],[])", 2);
}

#[test]
fn an_empty_list_inside_a_list_is_an_item() {
	is!("count([[] 1])", 2);
}
