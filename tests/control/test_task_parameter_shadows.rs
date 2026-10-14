//! Card param-shadows: a parameter named like a function of the program is the parameter inside the body, also in a
//! task's function (where the body is compiled on its own, the argument a closure)
use crate::is;

#[test]
fn a_parameter_named_like_a_function_is_the_parameter() {
	is!("inc = x => x + 1; f(inc) := { inc(3) }; await go f(inc)", 4);
	is!("inc = x => x + 1; twice = x => x * 2; f(inc) := { inc(3) }; await go f(twice)", 6);
	is!("inc = x => x + 1; f(inc) := { inc(3) }; f(inc)", 4);
}
