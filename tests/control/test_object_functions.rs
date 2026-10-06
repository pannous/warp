//! A function entry of an object literal (`f := it*2`, `f := x => …`, `f(x) := …`) is callable as `o.f(3)` / `o.f 3`;
//! the object's own fields are in scope of its body, `o = …` ends it
use crate::is;

#[test]
fn test_function_entries_are_callable() {
	is!("o = {a: 1, f := it*2}; o.f(3)", 6);
	is!("o = {f := it*2}; o.f 3", 6);
	is!("o = {a: 1, f := x => x*2}; o.f(3)", 6);
	is!("o = {a: 1, f = x => x*2}; o.f(3)", 6);
	is!("o = {a: 1, f(x, y) := x*y}; o.f(3, 4)", 12);
	is!("o = {a: 1, f := it*2}; o.f(3) + o.a", 7);
}

#[test]
fn test_fields_are_in_scope_of_function_entries() {
	is!("o = {a: 10, f := it*a}; o.f(3)", 30);
	is!("o = {a: 10, f := a => a*2}; o.f(3)", 6);
}
