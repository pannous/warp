// A function ending in the bare name of a nested function returns that function's result (P82: a bare name is a call);
// when that result is then called, the error names the form that returns the function itself, `function inc`
use crate::common::fails_with;
use crate::is;

#[test]
fn calling_the_result_of_a_bare_name_educates() {
	fails_with("def make(){ def inc(){ 1 }; inc }; c = make(); c()", "function inc");
}

#[test]
fn the_explicit_forms_work() {
	is!("def make(){ def inc(){ 1 }; function inc }; c = make(); c()", 1);
	is!("def make(){ def inc(){ 1 }; inc }; make()", 1); // the call, as P82 reads it
}
