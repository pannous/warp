// card method-bodies: a method body is lowered by every pass, as a function body is; passes before class_methods::lower
// skipped it (a class body was no child of Node::map_children)
use crate::is;

fn in_method(body: &str) -> String {
	format!("class C{{ n: int; fn f() := {body} }}; C(3).f()")
}

fn in_function(body: &str) -> String {
	format!("f() := {body}; f()")
}

/// The same body gives the same value in a method as in a function
fn same_in_method(body: &str) {
	assert_eq!(warp::wasm_emitter::eval(&in_method(body)), warp::wasm_emitter::eval(&in_function(body)), "{body}");
}

#[test]
fn phrases_in_a_method() {
	is!("class C{ n: int; fn f(xs) := count(xs.keep only positive) }; C(3).f([1, -2])", 1);
}

#[test]
fn filters_and_comprehensions_in_a_method() {
	is!(&in_method("count([1, 5, 9] where it > 3)"), 2);
	is!(&in_method("sum([x * 2 for x in [1, 2]])"), 6);
}

#[test]
fn assignments_in_a_method() {
	is!(&in_method("{ s = 0; s += 3; s }"), 3);
	is!(&in_method("{ x = 1; x++; x }"), 2);
}

#[test]
fn units_in_a_method() {
	same_in_method("2 km + 3 m");
	same_in_method("1 m < 2 m");
	same_in_method("5 ± 1");
}

#[test]
fn bodies_agree_with_functions() {
	for body in ["count([1, 5, 9] where it > 3)", "[1, 2, 3]#2", "2 + 3 * 4", "\"a\" + 2", "{ xs = [1]; xs.add(2); count(xs) }"] {
		same_in_method(body);
	}
}
