// The text of a map at run time: "{a:1 b:2}" like a list's "[1 2]", entries as key:value, in str, print and
// interpolation (it was the error "not a joinable item")
use warp::*;

#[test]
fn a_map_has_a_text() {
	is!("m = {a:1, b:2}; \"\\(m)\"", "{a:1 b:2}");
	is!("m = {}; m[\"a\"] = 1; m[\"b\"] = \"x\"; str(m)", "{a:1 b:x}");
	is!("xs = [1, [2, 3], \"a\"]; str(xs)", "[1 [2 3] a]");
}

#[test]
fn a_map_prints_and_is_the_value_of_print() {
	is!("p = {name:\"Joe\"}; print p; p.name", "Joe");
	is!("x = print({a:1, b:[1, 2]}); x.a", 1);
}
