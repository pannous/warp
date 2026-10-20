//! Every binding form in a function binds its own local, also of a variable the compiler made global (use_op reads op
//! before main binds it): the function never changes it without `global op` (card loop-variable-global)
use crate::is;

/// f's binder `body` binds an op of f's own; f gives its n and use_op still reads main's op.call 16
fn binding_in_function(body: &str) -> String {
	format!("use_op() := op.call\nop = {{call: 16}}\nf() := {{ n = 0; {body}; n }}\nf() + use_op()")
}

#[test]
fn a_loop_variable_stays_local() {
	is!(&binding_in_function("for op in [1, 2] { n += op }"), 19);
	is!("global op = 16\nf() := { n = 0; for op in [1, 2] { n += op }; n }\nf() + op", 19);
}

#[test]
fn destructured_names_stay_local() {
	is!(&binding_in_function("op, k = 3, 4; n = op"), 19);
}

#[test]
fn a_lambda_parameter_stays_local() {
	is!(&binding_in_function("m = [1, 2].map(op => op * 2); n = m#2"), 20);
}

#[test]
fn a_caught_error_stays_local() {
	is!(&binding_in_function("n = try (error \"x\") catch op { 7 }"), 23);
}

#[test]
fn a_match_binder_stays_local() {
	let program = binding_in_function("match Some(5) { Some(op) => n = op; None => 0 }");
	is!(&format!("type Option[T] = Some(T) | None\n{program}"), 21);
}
