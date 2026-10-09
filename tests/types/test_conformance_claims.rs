//! P177 (user, 2026-10-07): `class Square implements Shape` and `struct Square: Shape` are checked claims, like
//! `class Square{…} is Shape`: a compile error when Square lacks an operation of the trait Shape.
use crate::common::fails_with;
use crate::is;

const SHAPE: &str = "trait Shape { area(s) }";

#[test]
fn a_kept_claim_compiles() {
	is!(&format!("{SHAPE}; class Square implements Shape {{ side: int; area() := side * side }}; Square(3).area()"), 9);
	is!(&format!("{SHAPE}; struct Square: Shape {{ side: int; area() := side * side }}; Square(3).area()"), 9);
}

#[test]
fn a_claim_missing_an_operation_is_an_error() {
	fails_with(&format!("{SHAPE}; class Square implements Shape {{ side: int }}; 1"), "Square claims Shape but defines no area");
	fails_with(&format!("{SHAPE}; struct Square: Shape {{ side: int }}; 1"), "Square claims Shape but defines no area");
	fails_with(&format!("{SHAPE}; trait Named {{ name(s) }}; class Square implements Named, Shape {{ side: int; name() := \"sq\" }}; 1"), "Square claims Shape but defines no area");
}
