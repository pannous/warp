//! card runtime-function: `f == g` of function values only the run time knows compares their definitions as dump shows
//! them, up to the names of the parameters, and the values they captured
use crate::is;

#[test]
fn a_function_value_equals_itself() {
	is!("xs = [x => x + 1]; xs#1 == xs#1", true);
}

#[test]
fn equal_definitions_are_equal_functions() {
	is!("xs = [x => x + 1, y => y + 1]; xs#1 == xs#2", true);
	is!("f(x) := x + 1; g(y) := y + 1; xs = [&f, &g]; xs#1 == xs#2", true);
}

#[test]
fn different_definitions_are_different_functions() {
	is!("xs = [x => x + 1, z => z * 2]; xs#1 == xs#2", false);
	is!("xs = [x => x + 1, z => z * 2]; xs#1 != xs#2", true);
}

#[test]
fn closures_compare_their_captured_values() {
	is!("make(n) := x => x + n; a = make(1); b = make(1); a == b", true);
	is!("make(n) := x => x + n; a = make(1); c = make(2); a == c", false);
	is!("make(n) := x => x + n; make(1) == make(1)", true);
	is!("make(n) := x => x + n; make(1) == make(2)", false);
}

#[test] // a parameter named like a function is the parameter in its body: no static function comparison
fn a_parameter_shadows_a_function_of_its_name() {
	is!("class Q { k:int; name() := \"n\"; text() := \"q\" }; q = Q(1); f(name:any) := name is text; f(\"a\")", true);
}

#[test] // a loop variable named like a function holds the item where it is passed on, never the function
fn a_loop_variable_shadows_a_function_of_its_name() {
	is!("sample(x) := x; g(x) := x + 1; f(samples) := [g(sample) for sample in samples]; string(f([5, 6]))", "[6 7]");
	is!("sample(x) := x; g(x) := x + 1; total = 0; for sample in [5] { total = g(sample) }; total", 6);
}

#[test] // a parameter named like a class's method is a value in its body: no prefix call of the method
fn a_parameter_named_like_a_method_is_no_prefix_call() {
	is!("class Q { k:int; foo() := \"n\" }; q = Q(1); f(foo) := contains(foo, \"-\"); f(\"a-b\")", true);
}
