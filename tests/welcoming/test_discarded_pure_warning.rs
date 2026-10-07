//! wiki/mutable.md (card discarded-pure): a statement that drops a pure call's changed copy (`uppercase x`) changes
//! nothing, and assigning a mutating call (`y = x.upper!`) changes x too: both warn and name the intended forms
use crate::common::fails_with;
use crate::is;
use warp::diagnostic::{with_warning_mode, WarningMode};

#[test]
fn a_discarded_pure_call_warns() {
	with_warning_mode(WarningMode::Error, || fails_with("x=\"hi\"; uppercase x; x", "unused value of uppercase x"));
	with_warning_mode(WarningMode::Error, || fails_with("x=\"hi\"; x.upper; x", "x.upper! to change x"));
	with_warning_mode(WarningMode::Error, || fails_with("xs=[2,1]; sort(xs); xs", "unused value of sort xs"));
}

#[test]
fn an_assigned_mutating_call_warns() {
	with_warning_mode(WarningMode::Error, || fails_with("x=\"hi\"; y = x.upper!; y", "changes x too"));
}

#[test]
fn used_and_mutating_calls_do_not_warn() {
	with_warning_mode(WarningMode::Error, || is!("x=\"hi\"; y = uppercase x; y", "HI"));
	with_warning_mode(WarningMode::Error, || is!("x=\"hi\"; x.upper!; x", "HI"));
	with_warning_mode(WarningMode::Error, || is!("x=\"hi\"; uppercase x!; x", "HI"));
	with_warning_mode(WarningMode::Error, || is!("x=\"hi\"; uppercase x", "HI"));
}

#[test]
fn the_warned_program_still_runs() {
	is!("x=\"hi\"; uppercase x; x", "hi");
	is!("x=\"hi\"; y = x.upper!; x + y", "HIHI");
}
