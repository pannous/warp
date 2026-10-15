//! cards log-run, log-log: `log(x, 10)` and `log(x, 2)` are exact (log10, log2), also at run time; a list with a base
//! is the log of each item, not of the last one
use warp::wasm_emitter::eval;

fn printed(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn log_to_base_ten_and_two_is_exact() {
	assert_eq!(printed("log(1000, 10)"), "3");
	assert_eq!(printed("x = 1000; log(x, 10)"), "3");
	assert_eq!(printed("x = 8; log(x, 2)"), "3");
	assert_eq!(printed("round(log(81, 3))"), "4");
}

#[test]
fn a_list_with_a_base_is_the_log_of_each_item() {
	assert_eq!(printed("log([1 100], 10)"), "[0 2]");
	assert_eq!(printed("log([1, 8], 2)"), "[0 3]");
}

#[test]
fn log10_and_log2_broadcast_over_a_list_variable() {
	assert_eq!(printed("xs = [1 100]; log10(xs)"), "[0 2]");
	assert_eq!(printed("xs = [1 8]; log2 xs"), "[0 3]");
}
