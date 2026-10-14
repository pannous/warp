use crate::is;

#[test]
fn test_function_applied_to_a_negative_number() {
	is!("twice(x):=x*2;twice -3", -6);
}

#[test]
fn test_variable_minus_number_stays_subtraction() {
	is!("x=5;x -1", 4);
}

#[test]
fn test_builtin_name_bound_as_a_parameter_stays_subtraction() {
	is!("def pow(base, exp) := exp == 0 ? 1 : base * pow(base, exp-1)\npow(2, 10)", 1024);
}
