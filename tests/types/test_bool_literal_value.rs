//! 1 and 0 given to a bool are yes and no (P199) in the value of the expression too, not only in what the variable
//! holds afterwards (card bool-literal-value): `x: bool = 1` gives yes, a function giving its bool parameter gives yes.
use warp::Node;

fn is_yes(code: &str) {
	let value = warp::pipeline::eval(code);
	assert!(matches!(value.drop_meta(), Node::True), "{code} gives {value:?}, not yes");
}

#[test]
fn an_assignment_to_a_bool_gives_a_bool() {
	is_yes("x: bool = 1");
	is_yes("x: bool = 0; x = 1");
	is_yes("b = true; b = 1");
}

#[test]
fn a_function_giving_its_bool_parameter_gives_a_bool() {
	is_yes("f(b: bool) := b; f(1)");
	is_yes("f(b: bool) := b; y = f(1); y");
}
