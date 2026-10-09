//! Card effects-value: `effects of f` (`f.effects`) is a value anywhere: assigned, printed, interpolated, and as the
//! program's last statement it no longer replaces the program, whose prints run.

use crate::common::printed;
use crate::is;

fn printed_lines(code: &str) -> Vec<String> {
	printed(code).lines().filter(|line| !line.starts_with('»')).map(String::from).collect()
}

#[test]
fn effects_of_last_keeps_the_prints() {
	assert_eq!(printed_lines("print \"a\"\nsquare(x) := x*x\neffects of square"), ["a"]);
	is!("print \"a\"\nsquare(x) := x*x\neffects of square", "Pure");
}

#[test]
fn effects_of_as_a_value() {
	is!("square(x) := x*x\ne = effects of square\n\"square is \" + e", "square is Pure");
	is!("f(x) := print x\ne = f.effects\ne", "IO");
	is!("f(x) := print x\n\"f does \\(f.effects)\"", "f does IO");
	is!("g() := { puts \"hi\"; emit ask }\nes = effects of g\nes#2", "ask");
}
