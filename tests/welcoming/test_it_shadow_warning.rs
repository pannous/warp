// #23 (user 2026-10-03, "Warn on `it` shadowing"): a loop's `it` inside a function with an implicit `it` hides it, warned.
use crate::is;
use warp::diagnostic::{with_warning_mode, WarningMode};
use crate::common::fails_with;

#[test]
fn a_loop_it_still_shadows_the_function_it() {
	is!("f:={s=it; for 1..3 {s+=it}; s}; f(10)", 13);
}

#[test]
fn a_loop_it_hiding_the_function_it_warns() {
	with_warning_mode(WarningMode::Error, || fails_with("f:={s=it; for 1..3 {s+=it}; s}; f(10)", "hides"));
}

#[test]
fn a_loop_it_without_a_function_it_does_not_warn() {
	with_warning_mode(WarningMode::Error, || is!("s=0; for 1..3 {s+=it}; s", 3));
	with_warning_mode(WarningMode::Error, || is!("f:=it*2; f(4)", 8));
}
