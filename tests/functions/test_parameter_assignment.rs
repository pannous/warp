//! An annotated parameter is a declared place inside its function (P203/P204; card param-assign: `f(n: int) := { n =
//! "x"; n }; f(3)` gave 120, the code point): a literal of another type is a compile error, a value known only at run
//! time is checked when it is stored
use crate::common::fails_with;
use crate::is;

#[test]
fn assigning_an_annotated_parameter_checks_its_type() {
	fails_with("f(n: int) := { n = \"x\"; n }; f(3)", "n is declared int, cannot assign");
	fails_with("def f(n: int) { n = 2.5; n }; f(3)", "n is declared int, cannot assign float 2.5");
	fails_with("f(n: int, m: text) := { m = 1; m }; f(3, \"a\")", "m is declared text, cannot assign int 1");
	fails_with("f(n: int) := { t = [\"x\"]; n = t#1; n }; f(3)", "not an int");
	is!("f(n: int) := { n = 4; n }; f(3)", 4);
	is!("f(n: float) := { n = 4; n }; f(3)", 4.0);
	is!("f(n: int) := { n = 4; n }; n = \"x\"; f(3)", 4);
}
