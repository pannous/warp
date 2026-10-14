//! A bare name of a function without parameters is its call (decision reflection, P82) also where it is assigned or an
//! operand: `x = a` holds what a returns, `a + 1` adds to it (card assigning-bare)
use crate::is;
use warp::Node;

#[test]
fn test_assigned_bare_name_holds_the_result() {
	is!("def a():=1; x=a; type(x)", Node::Symbol("int".to_string()));
	is!("def a():=1; x=a; x", 1);
	is!("def a(){\"hi\"}; x=a; type(x)", Node::Symbol("text".to_string()));
}

#[test]
fn test_bare_name_as_an_operand() {
	is!("def a():=1; x=a+1; x", 2);
	is!("def a(){1}; y=3; y+a", 4);
	is!("a():=2; a*a", 4);
	is!("def a():=1; a < 2", true);
}

#[test]
fn test_function_reference_stays_the_function() {
	is!("def a():=1; f = function a; f()", 1);
}
