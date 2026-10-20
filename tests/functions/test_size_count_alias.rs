//! `size` is the global spelling of `count` unless a program defines `size` itself.
use crate::is;

#[test]
fn size_counts_collections() {
	is!("size([1, 2, 3])", 3);
	is!("size(\"🌍a\")", 2);
	is!("size({a: 1, b: 2})", 2);
}

#[test]
fn a_programs_size_function_takes_other_values() {
	is!("size(x) := x + 1; size(4)", 5);
}
