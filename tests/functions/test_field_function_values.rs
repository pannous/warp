//! A function stored in an object field or in a list in a field is a value that can be called (card function-values):
//! `s.f(4)` calls the closure field f holds, `s.fs#1(4)` the first function of the list field fs.
use crate::is;

#[test]
fn test_function_in_object_field() {
	is!("s = {f: x => x * 2}; s.f(4)", 8);
	is!("s = {f: (x => x * 2)}; s.f(4)", 8);
	is!("s = {f: x => x * 2, g: x => x + 1}; s.g(s.f(4))", 9);
	is!("s = {f: x => x * 2}; h = s.f; h(4)", 8);
	is!("n = 3; s = {f: x => x * n}; s.f(4)", 12);
	is!("s = {a: 3, f: x => x * a}; s.f(4)", 12);
	is!("s = {a: 3, f: x => x * a}; h = s.f; h(4)", 12);
}

#[test]
fn test_function_list_in_object_field() {
	is!("s = {fs: [x => x * 2]}; s.fs#1(4)", 8);
	is!("s = {fs: [x => x * 2, x => x + 1]}; s.fs#2(4)", 5);
	is!("s = {fs: [x => x * 2]}; t = s.fs#1; t(4)", 8);
}
