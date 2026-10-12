// card unknown-type: a variable's declared type naming no type is a warning, no longer silent; the program runs as if
// the type were not declared (the user's test_sinus declares `let z : tdouble`)
use crate::is;
use warp::diagnostic::take_warnings;

fn warns(code: &str, message: &str) -> bool {
	take_warnings();
	warp::wasm_emitter::eval(code);
	take_warnings().iter().any(|warning| warning.message.contains(message))
}

#[test]
fn an_unknown_variable_type_warns() {
	assert!(warns("p: foo = 2.5; p", "unknown type foo of p"));
	assert!(warns("x: flaot = 2.5; x", "unknown type flaot of x"));
	is!("p: foo = 2.5; p", 2.5);
}

#[test]
fn known_variable_types_stay_accepted() {
	is!("x: int = 3; x", 3);
	is!("x: int or text = 3; x", 3);
	is!("xs: ints = [1 2]; xs#2", 2);
	is!("x: float = 2.5; x", 2.5);
	is!("class Point { x: int }; p: Point = Point(1); p.x", 1);
	is!("x: int? = 3; x", 3);
	is!("xs: list of any = [1]; xs#1", 1);
	is!("f(xs: list of any) := xs#1; f([1])", 1);
	is!("tuple t = (1, 2, 3); t#3", 3);
	assert!(!warns("x: int = 3; x", "unknown type"));
}
