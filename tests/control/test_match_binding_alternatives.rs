// card binding-alternatives (matching roadmap step 6): `|` alternatives that bind the same names bind them, each as it
// matches; alternatives binding other names stay an error (typed code holes `$k: int`: card code-hole-types)
use crate::common::fails_with;
use crate::is;

const FACTOR: &str = "factor(n) := match n { data($x * 1) | data(1 * $x) => x; _ => 0 }\n";

#[test]
fn alternatives_bind_the_same_names() {
	is!(&format!("{FACTOR}factor(data(y * 1))"), "y");
	is!(&format!("{FACTOR}factor(data(1 * y))"), "y");
	is!(&format!("{FACTOR}factor(data(y * 3))"), 0);
	is!("match [2, 1] { [1, x] | [x, 1] => x; _ => 0 }", 2);
	is!("match [1, 5] { [1, x] | [x, 1] => x; _ => 0 }", 5);
	is!("match {b: 4} { {a: x} | {b: x} => x; _ => 0 }", 4);
}

#[test]
fn alternatives_binding_other_names_are_an_error() {
	fails_with(
		"match [1, 2] { [1, x] | [y, 1] => 0; _ => 0 }",
		"alternatives bind no names unless each binds the same ones",
	);
}
