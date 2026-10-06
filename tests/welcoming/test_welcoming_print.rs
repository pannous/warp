// `print` writes any number, text or character through its runtime text form; exact numbers that are no fixnum
// (ratios, big integers) have a text form like the host's; an exact variable assigned an f64 becomes a float. probes/print/
use crate::is;

#[test]
fn test_print_runtime_values() {
	// print gives nothing (issue #18)
	is!("x=3; print(x)", warp::Node::Empty);
	is!("f(x) := { print(x); x }; f(3)", 3);
	is!("y=2.5; print(y)", warp::Node::Empty);
	is!("s=\"hi\"; print(s)", warp::Node::Empty);
	is!("print(\"c\")", warp::Node::Empty);
	is!("y=print(\"c\"); y", warp::Node::Empty);
	is!("xs=[1,2]; print(xs)", warp::Node::Empty); // a list prints its str(xs) text (user, P32)
}

#[test]
fn test_exact_numbers_have_a_text_form() {
	is!("y=2.5; \"x\"+y", "x2.5");
	is!("y=2.5; y as string", "2.5");
	is!("y=-1/8; y as string", "-0.125");
	is!("y=10.05; y as string", "10.05");
	is!("y=1/3; \"v=\"+y", "v=1/3");
	is!("y=-7/3; y as string", "-7/3");
	is!("y=1.0/0.0; \"a\"+y", "a∞"); // P66: a float division by zero (an Int one is divide_by_zero)
	is!("y=0.0/0.0; \"a\"+y", "aNaN");
	is!("x=2^70; \"v\"+x", "v1180591620717411303424");
	is!("x=-(2^70); x as string", "-1180591620717411303424");
	is!("x=7; \"x\"+x", "x7");
}

#[test]
fn test_exact_variable_assigned_a_float_widens() {
	is!("import floor from \"m\"; x=10.0; x=floor(2.5); x+1", 3.0);
	is!("x=1; x=2.5 as float; x", 2.5);
	is!("x=10; x=floor(x/2)", 5);
}
