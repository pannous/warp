// `print` writes any number, text or character through its runtime text form; exact numbers that are no fixnum
// (ratios, big integers) have a text form like the host's; an exact variable assigned an f64 becomes a float. probes/print/
use warp::*;

use crate::common;

#[test]
fn test_print_runtime_values() {
	is!("x=3; print(x)", 3);
	is!("f(x) := { print(x); x }; f(3)", 3);
	is!("y=2.5; print(y)", 2.5);
	is!("s=\"hi\"; print(s)", "hi");
	is!("print(\"c\")", "c");
	is!("y=print(\"c\"); y", "c");
	is!("y=print(5); y+1", 6);
	common::fails_with("xs=[1,2]; print(xs)", "print of a List has no runtime text yet");
}

#[test]
fn test_exact_numbers_have_a_text_form() {
	is!("y=2.5; \"x\"+y", "x2.5");
	is!("y=2.5; y as string", "2.5");
	is!("y=-1/8; y as string", "-0.125");
	is!("y=10.05; y as string", "10.05");
	is!("y=1/3; \"v=\"+y", "v=1/3");
	is!("y=-7/3; y as string", "-7/3");
	is!("y=1/0; \"a\"+y", "a∞");
	is!("y=0/0; \"a\"+y", "aNaN");
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
