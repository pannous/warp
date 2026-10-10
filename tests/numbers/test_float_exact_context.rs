use crate::is;
use crate::common;

#[test]
fn test_float_read_in_exact_context_is_refused_loudly() {
	common::fails_with("f(x:float) := [10,20,30][x]; f(2.0)", "is a float where an exact Int is expected");
	common::fails_with("f(x:float) := x and 1; f(2.5)", "is a float where an exact Int is expected");
}

#[test]
fn test_explicit_cast_truncates_a_float() {
	is!("f(x:float) := x as int; f(2.7)", 2);
	is!("f(x:float) := int(x); f(-2.7)", -2);
	is!("f(x:float) := [10,20,30][int(x)]; f(2.0)", 30);
}

#[test]
fn test_ffi_float_result_in_exact_context_is_refused_loudly() {
	common::fails_with("import fabs from m; [10,20,30][fabs(0-2.5)]", "fabs is a float where an exact Int is expected");
	is!("import fabs from m; fabs(0-2.5) % 2", 0.5);
}

// card exact-reals (found by the kitchen sink): in a program evaluated at run time a constant expression of exact
// reals that is rational or a truth keeps its exact value; it was f64 (√2 * √2 == 2 false)
#[test]
fn a_rational_constant_of_exact_reals_in_a_runtime_program() {
	is!("xs = []; xs.add(√2 * √2 == 2); xs#1", true);
	is!("xs = []; r = √2 * √2; r == 2", true);
	is!("xs = []; r = √2 * √2; ok = r == 2; xs.add(ok); xs#1", true);
}

#[test]
fn test_a_declared_exact_variable_refuses_a_float_as_a_parameter_does() {
	const REFUSED: &str = "√(2) is a float where an exact rational is expected: declare it `number` to keep the float";
	common::fails_with("f(x: rational) := x; f(sqrt(2))", REFUSED);
	common::fails_with("x: rational = sqrt(2); x", REFUSED);
	common::fails_with("x: rational = 1; x = sqrt(2); x", REFUSED);
	common::fails_with("x as rational = sqrt(2)", REFUSED);
	common::fails_with("x: int = sqrt(2); x", "√(2) is a float where an exact int is expected");
}

#[test]
fn test_real_holds_roots_and_exact_unions_refuse_floats() {
	is!("x: real = sqrt(2); x * x > 1.99", true);
	is!("x: real = 1/3; x * 3", 1);
	is!("x: real = 1; x = sqrt(2); x > 1.4", true);
	is!("f(x: real) := x * 3; f(1/3)", 1);
	is!("f(x: real) := x > 1.4; f(sqrt(2))", true);
	common::fails_with("x: rational|int = sqrt(2)", "√(2) is a float where an exact rational is expected");
	is!("x: int|rational = 1/3; x * 3", 1);
	common::fails_with("x: int|text = sqrt(2)", "not an int or text");
	common::fails_with("f(x: int or text) := x; f(sqrt(2))", "f needs an int or text for parameter x");
}
