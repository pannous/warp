// A compound accumulator a function starts at `out = 0` widens to the floats of unknown origin it adds up, as
// `out = out + x` does (card compound-accumulator); an evident other kind stays a type error
use crate::is;
use crate::common::fails_with;

#[test]
fn a_compound_accumulator_takes_floats_of_unknown_origin() {
	is!("s(xs) := { out = 0; for x in xs { out += x }; out }; s([float(1.5), float(2)])", 3.5);
	is!("s(xs) := { out = 0; for x in xs { out += x }; out }; s([1, 2])", 3);
	is!("s(xs) := { out = 1; for x in xs { out *= x }; out }; s([float(1.5), float(2)])", 3.0);
	is!("xs = [1, \"a\", 2]; s = 0; for x in xs { if x is int { s += it } }; s", 3);
	fails_with("s=0; for i in 0..1 { s+=\"ab\" }; s", "type error: int + text");
}
