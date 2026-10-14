//! card dir-function, found editing samples/reflection.warp: the reflection words take a function itself, never call it
//! (P83, supervisor 2026-10-09). `dir(area)` was "undefined function: dir", and with a class in the program it called
//! the bare area, failing "area needs 2 arguments" at an earlier line
use crate::is;

const AREA: &str = "area(width: int, height: int) := width * height";
const WITH_A_CLASS: &str = "class P{x: int}\np = P(1)\narea(width: int, height: int) := width * height";
const REFLECTED: [&str; 4] = ["params", "signature", "body", "effects"];

#[test]
fn dir_of_a_function_lists_what_reflection_knows() {
	is!(&format!("{AREA}\ndir(area)"), warp::texts(REFLECTED.to_vec()));
	is!(&format!("{AREA}\ndir(&area)"), warp::texts(REFLECTED.to_vec()));
	is!(&format!("{WITH_A_CLASS}\ndir(area)"), warp::texts(REFLECTED.to_vec()));
}

#[test]
fn a_later_dir_leaves_an_earlier_line_alone() {
	is!(&format!("{WITH_A_CLASS}\nx = \"\\(area.params)\"\ndir(area)\nx"), "[\"width\" \"height\"]");
}

#[test]
fn the_type_of_a_function_is_function() {
	is!(&format!("{AREA}\ntype(area)"), "function");
	is!(&format!("{AREA}\ntype(&area)"), "function");
}
