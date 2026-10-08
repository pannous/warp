use crate::common::fails_with;
use crate::is;

#[test]
fn try_finally_without_catch_runs_the_cleanup() {
	is!("x = 0; try { x = 1 } finally { x = x + 10 }; x", 11);
}

#[test]
fn try_finally_keeps_the_guarded_value() {
	is!("try { 1 } finally { print \"done\" }", 1);
	is!("try:\n  1\nfinally:\n  2", 1);
}

#[test]
fn try_finally_keeps_an_uncaught_error() {
	fails_with("try { raise \"boom\" } finally { print \"done\" }", "boom");
}

#[test]
fn try_finally_runs_the_cleanup_after_a_trap() {
	is!("x = 0; try { 1/0 } finally { x = 5 }; x", 5);
	fails_with("try { 1/0 } finally { 2 }", "divide by zero");
}
