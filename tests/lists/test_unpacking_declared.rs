//! Unpacking into a declared name checks its type as `a = v` does (P203, P204; card destructure-unchecked: `a: int = 0;
//! a, b = 2.5, 6` gave 2.5 and `a, b = "x", 6` the code point 120): a literal that does not fit is a compile error,
//! a value known only at run time is checked when it is stored
use crate::common::fails_with;
use crate::is;

#[test]
fn unpacking_checks_declared_types() {
	fails_with("a: int = 0; a, b = 2.5, 6; a", "a is declared int, cannot assign float 2.5");
	fails_with("a: int = 0; a, b = \"x\", 6; a", "a is declared int");
	fails_with("a: int = 0; a, b = [2.5, 6]; a", "a is declared int, cannot assign float 2.5");
	fails_with("a: int = 0; t = \"x\"; a, b = t, 6; a", "not an int");
	fails_with("a: int = 0; xs = [2.5, 6]; a, b = xs; a", "whole number");
	is!("a: int = 0; a, b = 5, 6; a + b", 11);
	is!("a: float = 0; a, b = 5, 6; a + b", 11.0);
	is!("a: int = 0; xs = [5, 6]; a, b = xs; a + b", 11);
}
