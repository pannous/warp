//! P204 beyond int (card p204-beyond): a value whose type only the run time knows going into a place annotated float,
//! text, bool, list or a class is checked when it runs, as W0 does for assignment, binding and argument
use crate::common::fails_with;
use crate::is;

#[test]
fn any_into_annotated_places_is_checked_at_run_time() {
	fails_with("y: any = \"a\"; x: float = 0.5; x = y; x", "not a float");
	fails_with("y: any = \"a\"; x: float = y; x", "not a float");
	fails_with("y: any = 2.5; t: text = \"b\"; t = y; t", "not a text");
	fails_with("y: any = 2.5; t: text = y; t", "not a text");
	fails_with("y: any = 2; b: bool = y; b", "not a bool");
	fails_with("y: any = 3; xs: list = y; xs", "not a list");
	fails_with("class P { x: int }; y: any = 3; p: P = P(1); p = y; p", "not a P");
	fails_with("class P { x: int }; y: any = 3; f(p: P) := 1; f(y)", "not a P");
	fails_with("xs = [1, \"ab\"]; f(t: float) := t; f(xs#2)", "not a float");
	fails_with("xs = [1, \"ab\"]; f(t: text) := t; f(xs#1)", "not a text");
}

#[test]
fn values_of_the_annotated_type_pass() {
	is!("y: any = 3; x: float = y; x", 3.0);
	is!("y: any = \"ab\"; t: text = y; t", "ab");
	is!("y: any = \"a\"; f(s: text) := s; f(y)", "a");
	is!("y: any = yes; b: bool = y; b", true);
	is!("y: any = 1; b: bool = y; b", true);
	is!("y: any = [1, 2]; xs: list = y; count(xs)", 2);
	is!("class P { x: int }; y: any = P(2); p: P = y; p.x", 2);
	is!("class P { x: int }; f(p: P) := p.x; f(P(4))", 4);
	is!("xs = [1, \"ab\"]; f(t: text) := t; f(xs#2)", "ab");
	is!("xs = [1.5, \"ab\"]; f(t: float) := t; f(xs#1)", 1.5);
}

#[test]
fn any_into_a_declared_field_is_checked_at_run_time() {
	fails_with("class P{x:int}; y: any = \"a\"; p = P(y); p.x", "not an int");
	fails_with("class P{t:text}; y: any = 2.5; p = P(y); p.t", "not a text");
	fails_with("class P{x:float}; xs = [1, \"ab\"]; p = P(xs#2); p.x", "not a float");
	is!("class P{x:int}; y: any = 3; p = P(y); p.x", 3);
	is!("class P{t:text}; xs = [1, \"ab\"]; p = P(xs#2); p.t", "ab");
}
