//! A class without fields or methods lists none: a loop over them runs no round (card crash-index: it panicked)
use crate::is;

#[test]
fn a_loop_over_no_fields_runs_no_round() {
	is!("class E{}; for f in fields(E) {print f}; 3", 3);
}

#[test]
fn a_loop_over_no_methods_runs_no_round() {
	is!("class B{n:int}; n = 0; for m in methods(B) {n += 1}; n", 0);
}

#[test]
fn eval_of_a_loop_over_no_methods_runs_no_round() {
	is!("eval('class B{n:int}; for m in methods(B) {print m}; 3')", 3);
}
