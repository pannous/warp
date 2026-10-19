// card pattern-clauses (decision pattern-roadmap, notes/matching.md): a function defined by clauses whose parameters
// are patterns tries them in order, the first that matches answers; a plain name matches anything and binds it
use crate::common::fails_with;
use crate::is;

#[test]
fn literal_clauses_come_before_the_general_one() {
	is!("fib(0) := 0; fib(1) := 1; fib(n) := fib(n-1) + fib(n-2); fib(10)", 55);
	is!("sign(0) := \"zero\"; sign(n) := \"some\"; sign(0) + sign(4)", "zerosome");
}

#[test]
fn clauses_are_tried_in_order() {
	is!("f(n) := 1; f(0) := 2; f(0)", 1);
}

#[test]
fn list_clauses_take_a_list_apart() {
	is!("len([]) := 0; len([h, t...]) := 1 + len(t); len([4, 5, 6])", 3);
}

#[test]
fn code_clauses_rewrite_code() {
	is!("simplify(data($a * 1)) := a; simplify(n) := n; simplify(data(y * 1))", warp::Node::Symbol("y".into()));
	is!("simplify(data($a * 1)) := a; simplify(n) := n; simplify(7)", 7);
}

#[test]
fn several_parameters_match_together() {
	is!("mul(0, y) := 0; mul(x, 1) := x; mul(x, y) := x * y; mul(0, 9) + mul(5, 1) + mul(2, 3)", 11);
}

#[test]
fn a_parameter_name_leaves_a_global_of_the_same_name_alone() {
	is!("n = 7; f(0) := 1; f(n) := n + 1; f(3) + n", 11);
}

#[test]
fn a_value_no_clause_takes_is_no_case() {
	fails_with("f(0) := 1; f(1) := 2; f(5)", "no case");
}
