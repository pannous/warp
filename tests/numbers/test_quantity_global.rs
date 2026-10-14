// A global holding a quantity: `global cutoff = 1000Hz`, set again by a function (card quantity-global, found by warp-sound)
use crate::common;
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_global_holds_a_quantity() {
	assert_eq!(text_of("global x = 5m; x"), "5m");
	assert_eq!(text_of("global cutoff = 1000Hz; def f() { cutoff = 1.2kHz }; f(); cutoff"), "1.2/ms");
}

#[test]
fn a_plain_number_given_a_unit_says_how_to_start_it() {
	common::fails_with("cutoff = 1200; cutoff = 1.2kHz", "give its first value a unit");
}

#[test]
fn a_function_writes_a_global_in_its_units() {
	common::fails_with("global n = 1200; def f() { n = 1.2kHz }; f(); n", "give its first value a unit");
	common::fails_with("global n = 1200Hz; def f() { n = 5 }; f(); n", "n was 1/s, is given a plain number");
	common::fails_with("n = 1200; def f() { global n; n = 1.2kHz }; f(); n", "give its first value a unit");
}

#[test]
fn a_function_reads_a_global_with_its_unit() {
	assert_eq!(text_of("global n = 5m; def f() { n * 2 }; f()"), "10m");
	assert_eq!(text_of("n = 5m; def f() { global n; n * 2 }; f()"), "10m");
}
