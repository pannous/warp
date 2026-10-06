// Named arguments (user, P37): f(name=value) and f(name:value) set a parameter or a free variable of the body; a
// missing argument stays an error, a variable of the same name is never captured for a parameter
use crate::is;

#[test]
fn named_arguments_set_parameters_in_any_order() {
	is!("f(a, b) := a - b; f(b=1, a=5)", 4);
	is!("f(a, b) := a - b; f(b:1, a:5)", 4);
	is!("f(a, b) := a - b; f(5, b=1)", 4);
}

#[test]
fn named_arguments_set_free_variables_of_the_body() {
	is!("f y := y*y+v; f(y=2, v=3)", 7);
	is!("fun={x*y}; fun(x:2 y:3)", 6);
	is!("v=10; f(y) := y+v; f(y=1) + f(y=1, v=2)", 14);
}

#[test]
fn a_missing_or_unknown_argument_is_an_error() {
	crate::common::fails_with("x=7; f(x):=x*x; f()", "needs a value for parameter x");
	crate::common::fails_with("f(a, b) := a - b; f(b=1)", "needs a value for parameter a");
	crate::common::fails_with("f(a) := a; f(a=1, c=2)", "f has no parameter c");
}
