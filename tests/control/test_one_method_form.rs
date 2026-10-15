//! One method form (card one-method-form, notes/dispatch.md): a class-body `def x()` and an outside `def x(p:P)` are
//! the same method; every check runs over both declaration forms
use crate::is;
use warp::texts;

const FORMS: [&str; 2] = ["class P{n:int; def x():=n+1}", "class P{n:int}; def x(p:P):=p.n+1"];

fn each_form(code: &str, expected: impl Into<warp::Node> + Clone) {
	for form in FORMS {
		is!(&format!("{form}; {code}"), expected.clone().into());
	}
}

#[test]
fn both_forms_call_alike() {
	each_form("P(3).x()", 4);
	each_form("x(P(3))", 4);
	each_form("x P(3)", 4);
}

#[test]
fn both_forms_are_listed_alike() {
	each_form("[m.name for m in methods(P)]", texts(vec!["x"]));
	each_form("[m.name for m in functions(P(3))]", texts(vec!["x"]));
	each_form("dir(P(3))", texts(vec!["n", "x"]));
	each_form("count(methods of P(3))", 1);
}

#[test]
fn an_ancestors_outside_method_is_listed() {
	is!("class A{n:int}; class B extends A{m:int}; def x(a:A):=1; [m.name for m in methods(B)]", texts(vec!["x"]));
}

#[test]
fn a_parameter_typed_in_another_class_body_is_no_method_of_that_type() {
	is!("class P{m:int}; class Q{n:int; def f(o:P):=1}; count(methods(P))", 0);
}
