// Lists do not grow by index (user, 2026-10-04, P43): setting past the end, the empty list included, is a loud and
// catchable index error
use crate::is;

#[test]
fn setting_into_the_empty_list_is_an_index_error() {
	crate::common::fails_with("x=(); x#1=5", "index out of range");
	crate::common::fails_with("x=[]; x[0]=5", "index out of range");
}

#[test]
fn setting_past_the_end_is_catchable() {
	is!("x=[1]; try x#2=5 else 0", 0);
	is!("x=[]; try x#1=5 else 0", 0);
}
