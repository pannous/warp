//! Card type-non (notes/reflection.md "Rule": field → meta → reflection word): `x.type` of a value known only at run
//! time and without a `type` field or key is its run-time type, as `type(x)` says
use crate::common::fails_with;
use crate::is;
use warp::Node;

fn type_name(name: &str) -> Node {
	Node::Symbol(name.into())
}

#[test]
fn the_type_of_a_calls_result() {
	is!("def make() { 3 }; make().type", type_name("int"));
	is!("def f(o) { o.type }; f(1/3)", type_name("rational"));
	is!("def f(o) { o.type }; f([1 2])", type_name("list"));
}

#[test]
fn a_type_key_is_read_as_written() {
	is!("def f(o) { o.type }; f({type: 5})", 5);
}

#[test]
fn another_missing_field_stays_loud() {
	fails_with("def f(o) { o.zork }; f(3)", "no field zork");
}
