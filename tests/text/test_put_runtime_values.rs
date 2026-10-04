// puti, putl and putf write run-time values too, without a newline (they printed only literal numbers)
#![cfg(feature = "native")] // runs the warp binary
use crate::common::printed;

#[test]
fn puti_writes_a_loop_variable() {
	assert!(printed("for i in 1 to 3 : puti i").starts_with("123"));
	assert!(printed("i=4; if i>0 {puti(i)}").starts_with("4"));
}

#[test]
fn putf_writes_a_float_variable() {
	assert!(printed("x=2.5; putf x").starts_with("2.5"));
}
