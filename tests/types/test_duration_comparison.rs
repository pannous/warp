//! Durations compare (cards day-day, day-hours): `1 day == 1 day` is yes, not 1, and `1 day > 2 hours` holds on every
//! date, as a day is 23 to 25 hours and a month 28 to 31 days. Only an order that depends on the date is an error.
use crate::common::fails_with;
#[cfg(feature = "native")]
use crate::common::printed;
use crate::is;

#[test]
fn test_durations_order() {
	is!("1 day > 2 hours", true);
	is!("2 hours < 1 day", true);
	is!("3 hours >= 2 hours", true);
	is!("1 month > 27 days", true);
	is!("1 month < 32 days", true);
	is!("13 months > 12 months", true);
	is!("1 day 2 hours > 1 day", true);
	is!("2 days <= 1 day", false);
}

#[test]
fn test_durations_order_at_run_time() {
	is!("d = 1 day; d > 2 hours", true);
	is!("d = 3 hours; e = 2 hours; d >= e", true);
	is!("d = 1 month; d > 27 days", true);
	is!("d = 13 months; d > 12 months", true);
	is!("d = 2 days; d <= 1 day", false);
	fails_with("d = 1 day; d < 24 hours", "compare differently on different dates");
}

#[test]
fn test_an_order_that_depends_on_the_date_is_an_error() {
	fails_with("1 day < 24 hours", "compare differently on different dates");
	fails_with("1 day > 1470 minutes", "compare differently on different dates");
	fails_with("1 month > 30 days", "compare differently on different dates");
	fails_with("1 day == 24 hours", "equal on some dates and not on others");
}

#[cfg(feature = "native")]
#[test]
fn test_a_duration_comparison_prints_a_truth() {
	assert_eq!(printed("print(1 day == 1 day)").trim(), "yes");
	assert_eq!(printed("print(1 day > 2 hours)").trim(), "yes");
	assert_eq!(printed("print(1 day != 1 day)").trim(), "no");
}

/// card month-days: `<=` and `>=` hold when the difference can reach 0 but never cross it; calendar parts alone move
/// the date, not the hour, so daylight saving plays no part in `1 month <= 31 days`
#[test]
fn test_bounds_that_hold_on_every_date() {
	is!("1 month <= 31 days", true);
	is!("1 month >= 28 days", true);
	is!("1 month > 31 days", false);
	is!("1 month < 28 days", false);
	is!("1 month != 32 days", true);
	is!("1 day 1 hour >= 1 day", true);
	fails_with("1 month < 31 days", "cannot order durations");
	fails_with("1 day <= 24 hours", "cannot order durations");
}
