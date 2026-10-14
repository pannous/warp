//! Match arms by type (card match-type-arms): `int => …` tests the subject's type, `i: int => …` also binds it, a class
//! or enum name tests its instances; alternatives mix types and values (`int | text`, `0 or text`), the same type test
//! as `x is (int or text)` (user, matching roadmap: or is also the type union)
use crate::is;

#[test]
fn a_type_word_tests_the_type() {
	is!("kind_of(v) := match v { int => \"number\"; text => \"words\" }; kind_of(3)", "number");
	is!("kind_of(v) := match v { int => \"number\"; text => \"words\" }; kind_of(\"a\")", "words");
}

#[test]
fn a_named_type_binds_the_subject() {
	is!("describe(v) := match v { i: int => i + 1; t: text => t + \"!\" }; describe(3)", 4);
	is!("describe(v) := match v { i: int => i + 1; t: text => t + \"!\" }; describe(\"a\")", "a!");
}

#[test]
fn a_class_name_tests_its_instances() {
	is!(
		"class Circle{r:int}; class Square{s:int}; name(v) := match v { Circle => \"circle\"; Square => \"square\" }; name(Square(2))",
		"square"
	);
}

#[test]
fn types_and_values_as_alternatives() {
	is!("f(v) := match v { int | text => \"plain\"; _ => \"other\" }; f(\"a\")", "plain");
	is!("f(v) := match v { int or text => \"plain\"; _ => \"other\" }; f([1])", "other");
	is!("f(v) := match v { 0 or text => \"zero or text\"; _ => \"other\" }; f(0)", "zero or text");
	is!("f(v) := match v { 0 or text => \"zero or text\"; _ => \"other\" }; f(\"a\")", "zero or text");
	is!("f(v) := match v { 0 or text => \"zero or text\"; _ => \"other\" }; f(1)", "other");
}

#[test]
fn is_tests_a_type_union() {
	is!("x = 3; x is (int or text)", true);
	is!("x = [3]; x is (int or text)", false);
}

#[test]
fn a_variable_named_like_a_type_stays_a_value() {
	is!("text = 3; match 3 { text => \"three\"; _ => \"other\" }", "three");
}
