//! Card introspection-reflection: a function's reflection words are read off its function value at run time, for
//! user functions and library words alike (notes/reflection_plain.md)
use crate::is;

#[test]
fn a_library_words_return_type_comes_from_its_signature() {
	is!("floor.return_type", "int");
	is!("sqrt.returns", "number");
}

#[test]
fn a_user_functions_words_come_from_its_value() {
	is!("sq(x) := x * x\nsq.parameters == sq.params", true);
	is!("sq(x) := x * x\ntype(sq)", "function: (x) -> int");
	is!("sq(x) := x * x\nh(f: function) := f.signature\nh(sq) == sq.signature", true);
}

#[test]
fn a_functions_body_is_code_from_its_value() {
	is!("sq(x) := x * x\nh(f: function) := f.body\nh(sq) is code", true);
	is!("f(x) := { y = x + 1; y * 2 }\nf.body is code", true);
}
