//! A variable holding a time or a duration is a time wherever it is used, as the written value is (card variable-time)
use crate::common::fails_with;
use crate::is;

#[test]
fn durations_with_the_same_days_order_by_their_exact_part() {
	is!("2 hours < 3 hours", true);
	is!("90 minutes > 1 hour", true);
	is!("1 day + 2 hours < 1 day + 3 hours", true);
	fails_with("1 day < 25 hours", "cannot order durations");
	is!("x = 2 hours; y = 3 hours; x < y", true);
	is!("x = 2 hours; y = 3 hours; x >= y", false);
}

#[test]
fn a_duration_variable_compares_by_value() {
	is!("x = 2 hours; x*3 == x*3", true);
	is!("x = 2 hours; x*3 == 6 hours", true);
	is!("x = 2 hours; x*3 != x", true);
}

#[test]
fn a_field_of_a_grouped_time_variable() {
	is!("x = now; ((x + 1 hour).hour - x.hour + 24) % 24", 1);
}
