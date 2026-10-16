// Rust references (card rust-refs, notes/rust_reading.md): `&x`, `&mut x` and `*r` read as the value itself, because
// warp values need no references; ownership and borrowing stay out of scope
use crate::is;

#[test]
fn a_dereference_reads_the_value() {
	is!("let x = 3; let r = &x; *r", 3);
	is!("let x = 3; let r = &x; *r + 1", 4);
	is!("let x = 3; let r = &x; 2 * *r", 6);
	is!("let x = 3; *(&x)", 3);
}

#[test]
fn a_mutable_reference_reads_the_value() {
	is!("var x = 3; let r = &mut x; *r + 1", 4);
	is!("let mut x = 3; let r = &mut x; *r", 3);
}

#[test]
fn references_of_values_and_nested_references() {
	is!("let r = &5; *r", 5);
	is!("let x = 3; let r = &&x; **r", 3);
	is!("let xs = [1, 2, 3]; let r = &xs; r.1", 2);
}

#[test]
fn a_reference_parameter_and_argument() {
	is!("fn inc(a: &i32) -> i32 { *a + 1 } inc(&4)", 5);
	is!("fn first(xs: &[i32]) -> i32 { xs.0 } first(&[7, 8])", 7);
}

#[test]
fn star_still_destructures_and_spreads() {
	is!("a, *rest = [1, 2, 3]; rest", warp::ints(vec![2, 3]));
	is!("def total(*xs) := xs.sum; total(1, 2, 3)", 6);
	is!("def named(**kw) := kw.a; named(a=4)", 4);
	is!("def add(a, b) := a + b; xs = [1, 2]; add(*xs)", 3);
}
