// Precomputed and precompiled paths (wiki/charged.md §3, released 2026-10-05, package 3): the compiler may compute a
// pure part earlier; whatever reads shared state or has effects runs at every call.
use warp::*;

#[test]
fn a_function_reading_a_global_is_not_pure() {
	is!("global y=1; f() := y*2; effects of f", Node::Symbol("State".into()));
	is!("y=3; def z(){ global y; y*y }; effects of z", Node::Symbol("State".into()));
	is!("y=1; f() := y*2; effects of f", Node::Symbol("Pure".into()));
	is!("global y=1; f(y) := y*2; effects of f", Node::Symbol("Pure".into()));
}
