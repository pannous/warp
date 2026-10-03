//! `T {x: 1}` with a space constructs a declared type T like the glued `T{x: 1}` (D4); an undeclared word before a block
//! stays data (open decision 41)
use crate::common::fails_with;
use warp::*;

#[test] // samples/raytracer.wasp, samples/particles.wasp
fn test_spaced_construction_of_a_declared_type() {
	is!("class V { x: float, y: float }; v = V { x: 1, y: 2 }; v.y", 2);
	is!("class V { x: float, y: float }; vs = [V { x: 1, y: 2 }, V { x: 3, y: 4 }]; (vs#2).x", 3);
	is!("class V { x: float, y: float }; def f(a, b) := V { x: a, y: b }; f(5, 6).x", 5);
}

#[test] // samples/raytracer.wasp: `def add(a, b) := vec(a.x + b.x, …)` reads fields of values typed only at run time
fn test_field_of_a_declared_type_reads_at_run_time() {
	is!("class V { x: float, y: float }; def f(a, b) := V(a, b); v = f(5, 6); v.x", 5);
	is!("class V { x: float, y: float }; def g(v) := v.x * 2; g(V(3, 4))", 6);
	fails_with("class V { x: float, y: float }; x=[1]; x.y", "no field y");
	fails_with("x=\"hello\"; x.shout", "undefined function: shout");
}
