//! Card reflect-inferred (notes/reflection.md step 2): reflection of instances only the run time knows (a call's
//! result) and of extension methods, which `methods` and `dir` list among the class's own
use crate::is;
use warp::Node;

const MAKE: &str = "class P { x: int; def sum() { x } }; def make() { P(1) }";

#[test]
fn the_class_of_a_calls_result() {
	is!(&format!("{MAKE}; make().class"), Node::Symbol("P".into()));
	is!(&format!("{MAKE}; make().type"), Node::Symbol("P".into()));
	is!(&format!("{MAKE}; p = make(); p.class"), Node::Symbol("P".into()));
}

#[test]
fn a_field_named_type_is_read_as_written() {
	is!("class Q { type: int }; def make() { Q(7) }; make().type", 7);
}

#[test]
fn extension_methods_are_listed() {
	let twice = "fun P.twice() = this.x * 2";
	is!(&format!("{MAKE}; {twice}; [m.name for m in P.methods]"), warp::texts(vec!["sum", "twice"]));
	is!(&format!("{MAKE}; {twice}; [m.name for m in P(3).methods]"), warp::texts(vec!["sum", "twice"]));
	is!(
		"class P { x: int }; extension P { func twice() -> int { self.x * 2 }; func three() { 3 } }; dir(P(1))",
		warp::texts(vec!["x", "twice", "three"])
	);
}
