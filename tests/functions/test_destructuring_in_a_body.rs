// Cards destructure-return and destructure-exp (samples/neural_net.wasp): `(h, o) = fw()` in a function body is a
// statement, so the body ends with the value of its last line (it was a data block holding both values, {0.4 0.4})
use crate::is;

#[test]
fn a_body_with_destructuring_gives_its_last_value() {
	is!("t() := { (h, o) = (1, 2); o }; t()", 2);
	is!("def fw() { return ([1.0], 0.4) }; def t(target) { (h, o) = fw(); o }; t(1)", 0.4);
	is!("def fw() { return ([1.0], 0.25) }; def t(target) { (h, o) = fw(); target - o }; t(1)", 0.75);
}
