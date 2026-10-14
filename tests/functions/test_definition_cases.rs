// card definition-cases (user decision definition-cases, notes/decisions.md): a second definition of the same shape is
// the error "fib is defined twice" with a fix pointing at a match; cases are not joined. Overloads by parameter count or
// by parameter type are unaffected (test_arity_overloads.rs).
use crate::common::fails_with;
use crate::is;

#[test]
#[ignore = "cleanup-edge: cleanup-cut-definition-cases: the defined-twice diagnostic is cut (cleanup-cuts-2)"]
fn definitions_by_cases_are_defined_twice() {
	fails_with("fib(0) := 0; fib(1) := 1; fib(n) := fib(n-1) + fib(n-2); fib(10)", "fib is defined twice");
	fails_with("f(x) := 1; f(y) := 2; f(3)", "f is defined twice");
}

#[test]
#[ignore = "cleanup-edge: cleanup-cut-definition-cases: the defined-twice diagnostic is cut (cleanup-cuts-2)"]
fn the_fix_joins_the_cases_in_a_match() {
	fails_with("fib(0) := 0; fib(n) := n; fib(3)", "fib(n) := match n {");
}

#[test]
#[ignore = "cleanup-edge: cleanup-cut-definition-cases: the defined-twice diagnostic is cut (cleanup-cuts-2)"]
fn a_literal_parameter_alone_is_refused() {
	fails_with("f(0) := 1; f(3)", "a parameter of f is a name");
}

#[test]
fn overloads_by_type_and_count_stay() {
	is!("f(x:int) := 1; f(x:text) := 2; f(\"a\")", 2);
	is!("f(a) := a; f(a,b) := a+b; f(1) + f(2,3)", 6);
}

#[test]
fn the_match_the_fix_names_works() {
	is!("fib(n) := match n { 0: 0; 1: 1; _: fib(n-1) + fib(n-2) }; fib(10)", 55);
}

#[test]
fn definitions_without_parameters_have_no_literal() {
	is!("make() := 3; make()", 3);
	is!("def outer() { 4 }; outer()", 4);
	is!("fun g(): 2; g()", 2);
}
