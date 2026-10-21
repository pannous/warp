//! card type-function: a function value carries its signature at run time, so type() gives one answer for one value
//! however it is written: a lambda, a list item, a field, a parameter, a returned function
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

const DOUBLING: &str = "\"function: (x) -> int\"";

#[test]
fn every_spelling_of_a_function_has_its_signature_as_type() {
	assert_eq!(printed("f = x => x*2; type(f)"), DOUBLING);
	assert_eq!(printed("type(x => x*2)"), DOUBLING);
	assert_eq!(printed("fs = [x => x*2]; type(fs#1)"), DOUBLING);
	assert_eq!(printed("m = {f: x => x*2}; type(m.f)"), DOUBLING);
	assert_eq!(printed("g(h) := type(h); g(x => x*2)"), DOUBLING);
	assert_eq!(printed("mk() := x => x*2; type(mk())"), DOUBLING);
}

#[test]
fn a_builtin_passed_on_has_its_builtin_type() {
	assert_eq!(printed("g(h) := type(h); g(function floor)"), "\"builtin: (x:number) -> int\"");
}

#[test]
fn a_function_value_has_its_params_at_run_time() {
	assert_eq!(printed("fs = [x => x*2]; fs#1.params"), "['x']");
	assert_eq!(printed("g(h) := h.params; f(a, b) := a; g(f)"), "['a' 'b']");
	assert_eq!(printed("m = {params: 3}; m.params"), "3");
}

#[test]
fn other_values_keep_their_type() {
	assert_eq!(printed("x = [1]; type(x#1)"), "class int");
}
