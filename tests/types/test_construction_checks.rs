// A construction by call, `P(0.5)` or `P(x:1)`, is checked like the braces `P{x:0.5}`: the fields fit their declared
// types, named arguments name fields (card int-field)
use crate::is;

#[test]
fn a_call_construction_checks_its_fields_as_the_braces_do() {
	crate::common::fails_with("class P{x:int}; P(0.5)", "P.x is int, got float 0.5");
	crate::common::fails_with("class P{x:int}; P(\"a\")", "P.x is int, got");
	is!("class P{x:int; y:int}; P(y:2, x:1).x", 1);
	is!("class P{x:int; y:int=5}; P(x:1).y", 5);
	crate::common::fails_with("class P{x:int}; P(x:0.5)", "P.x is int, got float 0.5");
}
