//! Date arithmetic at run time (card run-time-dates, stage B2, notes/dates_at_run_time.md): `d + 1 day` with variables,
//! `time - time`, durations that add and scale, juxtaposed `1 month 2 days`. Zoned arithmetic waits for B3.
use crate::common::fails_with;
use crate::is;
use warp::node::Node;
use warp::time::calendar::Date;
use warp::time::{Duration, Time};
use warp::wasm_emitter::eval;

fn date(year: i64, month: i64, day: i64) -> Node {
	Node::data(Time::Date(Date::new(year, month, day).expect("a date")))
}

#[test]
fn test_a_date_plus_a_duration() {
	assert_eq!(eval("d = 2024-02-28; print d; d + 1 day"), date(2024, 2, 29));
	assert_eq!(eval("d = 2024-02-28; print d; 2 days + d"), date(2024, 3, 1));
	assert_eq!(eval("d = 2024-03-01; print d; d - 1 day"), date(2024, 2, 29));
	assert_eq!(eval("d = 2024-01-15; n = 3; print n; d + n * 1 month"), date(2024, 4, 15));
	assert_eq!(eval("d = 2024-01-31; print d; d + 1 year"), date(2025, 1, 31));
}

#[test]
fn test_month_overflow_is_rejected_at_run_time() {
	fails_with("d = 2024-01-31; print d; d + 1 month", "does not exist");
}

#[test]
fn test_a_local_time_and_an_instant_plus_a_duration() {
	let expected = Node::data(warp::time::calendar::parse_literal("2024-01-31T11:30").expect("local"));
	assert_eq!(eval("t = 2024-01-31T10:00; print t; t + 90 minutes"), expected);
	let expected = Node::data(warp::time::calendar::parse_literal("2024-02-01T00:30:00Z").expect("instant"));
	assert_eq!(eval("t = 2024-01-31T23:00:00Z; print t; t + 90 minutes"), expected);
	let expected = Node::data(warp::time::calendar::parse_literal("2024-02-01T00:30").expect("local"));
	assert_eq!(
		eval("t = 2024-01-31T23:00; print t; t + 1 day + 90 minutes - 1 day"),
		eval("t = 2024-01-31T23:00; print t; t + 90 minutes")
	);
	assert_eq!(eval("t = 2024-01-31T23:00; print t; t + 90 minutes"), expected);
}

#[test]
fn test_time_minus_time() {
	is!("a = 2024-03-01; b = 2024-02-01; print a; a - b", 29);
	assert_eq!(
		eval("a = 2024-01-01T10:00; b = 2024-01-01T08:30; print a; a - b"),
		Node::data(Duration { nanos: 5_400_000_000_000, ..Duration::default() })
	);
	assert_eq!(
		eval("a = 2024-01-01T08:00:00Z; b = 2024-01-01T08:00:01.5Z; print a; a - b"),
		Node::data(Duration { nanos: -1_500_000_000, ..Duration::default() })
	);
	fails_with("a = 2024-01-01; b = 2024-01-01T10:00; print a; a - b", "no implicit conversion between kinds of time");
}

#[test]
fn test_durations_add_and_scale() {
	assert_eq!(eval("a = 1 month; b = 2 days; print a; a + b"), Node::data(Duration { months: 1, days: 2, nanos: 0 }));
	assert_eq!(eval("a = 1 month; b = 2 days; print a; a - b"), Node::data(Duration { months: 1, days: -2, nanos: 0 }));
	assert_eq!(
		eval("a = 3 hours; n = 2; print n; a * n"),
		Node::data(Duration { nanos: 21_600_000_000_000, ..Duration::default() })
	);
	assert_eq!(
		eval("a = 3 hours; n = 2; print n; n * a"),
		Node::data(Duration { nanos: 21_600_000_000_000, ..Duration::default() })
	);
}

#[test]
fn test_juxtaposed_durations_run() {
	assert_eq!(eval("m = 1 month 2 days; print m; m"), Node::data(Duration { months: 1, days: 2, nanos: 0 }));
	assert_eq!(eval("d = 2024-01-15; print d; d + 1 month 2 days"), date(2024, 2, 17));
}

#[test]
#[cfg(feature = "native")]
fn test_arithmetic_prints() {
	use crate::common::printed;
	assert_eq!(
		printed("d = 2024-02-28; for i in 1..4 { d = d + 1 day; print d }"),
		"2024-02-29\n2024-03-01\n2024-03-02\n"
	);
}

/// Constants fold at compile time (time.rs fold_constants); numbers and truths around them are the run's
#[test]
fn test_constant_times_fold_and_the_run_does_the_rest() {
	assert_eq!(eval("2024-03-01 - 1 day"), date(2024, 2, 29));
	is!("2024-02-29.month + 1", 3);
	is!("date(2024, 2, 29).day * 2", 58);
	is!("2024-02-29 < 2024-03-01 and 2024-01-01 < 2024-02-01", 1);
	is!("1 day != 2 days", 1);
	assert_eq!(eval("2024-01-01; 2024-02-01"), date(2024, 2, 1));
	assert_eq!(eval("2 * 1 day"), Node::data(Duration { months: 0, days: 2, nanos: 0 }));
}
