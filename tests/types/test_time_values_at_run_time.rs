//! Time values at run time (card run-time-dates, notes/dates_at_run_time.md): date, local time and instant literals in
//! a program that prints, loops or keeps them in variables are run-time values, Kind::Time nodes, no longer refused
use crate::common::fails_with;
use crate::is;
use warp::node::Node;
use warp::time::calendar::Date;
use warp::time::Time;
use warp::wasm_emitter::eval;

#[test]
#[cfg(feature = "native")] // printed runs the warp binary
fn test_time_literals_print() {
	use crate::common::printed;
	assert_eq!(printed("print 2024-02-29"), "2024-02-29\n");
	assert_eq!(printed("print 2024-01-31T10:30"), "2024-01-31T10:30\n");
	assert_eq!(printed("print 2024-01-31T10:30:15.5Z"), "2024-01-31T10:30:15.5Z\n");
	assert_eq!(printed("for i in 1..3 { print 2024-02-29 }"), "2024-02-29\n2024-02-29\n");
	assert_eq!(printed("print 2024-02-29 + 1 day"), "2024-03-01\n"); // a constant expression folds
}

#[test]
fn test_a_time_reads_back() {
	assert_eq!(eval("print 1; 2024-02-29"), Node::data(Time::Date(Date::new(2024, 2, 29).expect("a date"))));
}

#[test]
fn test_a_time_joins_a_text() {
	is!("x = 1; print x; \"on \" + 2024-02-29", "on 2024-02-29");
	is!("d = 2024-01-31T10:30; print d; str(d)", "2024-01-31T10:30");
}

#[test]
fn test_fields_at_run_time() {
	is!("d = 2024-02-29; print d; d.month", 2);
	is!("d = 2024-02-29; print d; d.year * 10000 + d.month * 100 + d.day", 20240229);
	is!("d = 2024-02-29; print d; d.weekday", 4); // a Thursday
	is!("d = 2024-02-29; print d; d.day_of_year", 60);
	is!("t = 2024-01-31T10:30:15; print t; t.hour + t.minute + t.second", 55);
	is!("t = 2024-01-31T10:30Z; print t; t.epoch_seconds", 1706697000);
	fails_with("d = 2024-02-29; print d; d.hour", "date has no hour");
}

/// An instant's wall clock is the environment's time zone (user, 2026-10-10: "provide the default local from the
/// environment through the runtime"), daylight saving included: the host word local_offset
#[test]
#[cfg(feature = "native")] // the browser's zone is the page's, not warp::time::local_offset's
fn test_an_instant_reads_the_local_wall_clock() {
	for (instant, written) in [(1706697000, "2024-01-31T10:30Z"), (1719829800, "2024-07-01T10:30Z")] {
		let local_minutes =
			(instant + warp::time::local_offset(instant as i128 * 1_000_000_000)).rem_euclid(86_400) / 60;
		is!(&format!("t = {written}; print t; t.hour * 60 + t.minute"), local_minutes);
		is!(&format!("{written}.hour * 60 + {written}.minute"), local_minutes);
	}
}

#[test]
fn test_times_compare_at_run_time() {
	is!("a = 2024-01-01; b = 2024-03-01; print a; a < b", true);
	is!("a = 2024-01-01T10:00; b = 2024-01-01T09:00; print a; a > b", true);
	fails_with("a = 2024-01-01; b = 2024-01-01T10:00; print a; a < b", "no implicit conversion between kinds of time");
}

/// Stage B1b: durations and zoned times are run-time values too, their days, months, zone and offset in the info bits
#[test]
#[cfg(feature = "native")] // printed runs the warp binary
fn test_durations_and_zoned_times_print() {
	use crate::common::printed;
	assert_eq!(printed("d = 1 day; print d"), "1 day\n");
	assert_eq!(
		printed("d = 1 month + 2 days + 3 hours + 4 minutes + 5 seconds; print d"),
		"1 month 2 days 3 hours 4 minutes 5 seconds\n"
	);
	assert_eq!(printed("d = 2024-01-01T00:00:01.5Z - 2024-01-01T00:00:03Z; print d"), "-1.5 seconds\n");
	assert_eq!(printed("z = 2024-01-31T10:00[Europe/Berlin]; print z"), "2024-01-31T10:00+01:00[Europe/Berlin]\n");
	assert_eq!(
		printed("z = 2030-07-01T09:30:15[America/New_York]; print z"),
		"2030-07-01T09:30:15-04:00[America/New_York][_tzdata=warp-2026a]\n"
	);
}

#[test]
fn test_durations_and_zoned_times_read_back() {
	use warp::time::Duration;
	assert_eq!(eval("print 1; 90 minutes"), Node::data(Duration { nanos: 5_400_000_000_000, ..Duration::default() }));
	assert_eq!(eval("print 1; 2 months + 3 days"), Node::data(Duration { months: 2, days: 3, nanos: 0 }));
	is!("d = 1 day; print d; \"in \" + d", "in 1 day");
	is!("z = 2024-01-31T10:00[Europe/Berlin]; print z; str(z)", "2024-01-31T10:00+01:00[Europe/Berlin]");
}

#[test]
fn test_zoned_fields_at_run_time() {
	is!("z = 2024-07-01T10:30[Europe/Berlin]; print z; z.hour * 60 + z.minute", 630);
	is!("z = 2024-07-01T10:30[Europe/Berlin]; print z; z.offset", 7200);
	is!("z = 2024-01-31T23:30[America/New_York]; print z; z.day", 31);
	fails_with("d = 2 days; print d; d.year", "duration has no field year");
}

#[test]
fn test_zoned_times_and_durations_compare_at_run_time() {
	is!("a = 2024-01-31T10:00[Europe/Berlin]; b = 2024-01-31T10:00[Europe/London]; print a; a < b", true);
	fails_with("a = 1 day; b = 2 days; print a; a < b", "cannot order durations");
}
