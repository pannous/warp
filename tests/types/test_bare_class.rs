// card bare-class (decision type-print): `class int` without a body is the int type, as `function f` is the function, so
// a printed `class time` pasted back is time; an unknown class word is an error naming the nearest type
use crate::is;

#[test]
fn a_bare_class_word_is_the_type() {
	is!("class int == int", true);
	is!("x = class text; x == text", true);
	is!("x = class time; x == time", true);
	is!("class Int == int", true);
	is!("f(t: type) := t == int; f(class int)", true);
}

#[test]
fn an_unknown_bare_class_names_the_nearest_type() {
	crate::common::fails_with("class Strng", "did you mean string?");
	crate::common::fails_with("class Person{name}; class Persn", "did you mean Person?");
	crate::common::fails_with("class Foo", "unknown type Foo");
}

#[test]
fn a_bodyless_class_used_elsewhere_still_declares_it() {
	is!("sealed class Shape\nclass Circle(val r: Int) : Shape()\nCircle(2).r", 2);
	is!("class Foo; int bar(){ 42 }; Foo().bar()", 42);
}
