// card unknown-type: a variable's declared type must name a type, as a parameter's must; `p: foo = 2.5` is an error
use crate::common::fails_with;
use crate::is;

#[test]
fn an_unknown_variable_type_is_an_error() {
	fails_with("p: foo = 2.5; p", "unknown type foo of p");
	fails_with("x: flaot = 2.5; x", "unknown type flaot of x");
}

#[test]
fn known_variable_types_stay_accepted() {
	is!("x: int = 3; x", 3);
	is!("x: int or text = 3; x", 3);
	is!("x: float = 2.5; x", 2.5);
	is!("class Point { x: int }; p: Point = Point(1); p.x", 1);
	is!("x: int? = 3; x", 3);
	is!("xs: list of any = [1]; xs#1", 1);
	is!("f(xs: list of any) := xs#1; f([1])", 1);
}
