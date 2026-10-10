// card int-declaration: a declared int never holds a fraction (wiki/Footguns.md: no silent lossy conversion). A literal is
// refused at compile time; a fraction that arrives at run time (`x += 0.5`, a variable holding 0.5) is a loud run-time error
use crate::common::fails_with;
use crate::is;

const NOT_WHOLE: &str = "an int must be a whole number";

#[test]
fn a_declared_int_refuses_a_fraction_at_run_time() {
	fails_with("x:int = 3; x += 1/2; x", NOT_WHOLE);
	fails_with("x:int = 3; x *= 3/2; x", NOT_WHOLE);
	fails_with("x:int = 3; y = 1/2; x = y; x", NOT_WHOLE);
	// a decimal is a float (decision exact-default), refused while compiling
	fails_with("x:int = 3; x += 0.5; x", "is a float where an exact int is expected");
	fails_with("x:int = 3; x = x / 2; x", NOT_WHOLE);
}

#[test]
fn a_declared_int_takes_whole_results() {
	is!("x:int = 3; x *= 2; x", 6);
	is!("x:int = 3; x /= 2; x", 1); // `/=` keeps an int an int
	is!("x = 3; x += 0.5; x", 3.5);
}

#[test]
fn an_int_field_refuses_a_fraction() {
	fails_with("class P{x:int}; p = P(1/2); p.x", NOT_WHOLE);
	fails_with("class P{x:int}; p = P(1); p.x = 1/2; p.x", NOT_WHOLE);
	fails_with("class P{x:int}; p = P(1); p.x += 1/2; p.x", NOT_WHOLE);
	fails_with("class P{x:int}; p = P(0.5); p.x", "P.x is int, got float 0.5"); // decision exact-default
	is!("class P{x:int}; p = P(1); p.x = 2; p.x", 2);
}
