//! A main-level object or list a function reads keeps its type there: `k.a` is the Int 10, not text
use warp::is;

#[test]
fn test_a_captured_object_field_keeps_its_kind() {
	is!("k = {a: 10}; g := it*k.a; g(3)", 30);
	is!("k = {a: 10, b: 2}; g(x) := x*k.b; g(3)", 6);
	is!("k = {a: 1.5}; g := it*k.a; g(2)", 3.0);
	is!("k = {a: 10}; g(x) := x + k.a; g(3)", 13);
}

#[test]
fn test_a_captured_list_element_keeps_its_kind() {
	is!("k = [1, 2]; g := it*k#2; g(3)", 6);
	is!("k = [1.5, 2.5]; g(x) := x*k#1; g(2)", 3.0);
}
