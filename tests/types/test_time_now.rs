//! A bare `time` is the type, `time.now` the local time of day, as `now.time` (cards print-time, time-now, user
//! decision time-word); two clock reads compare (card time-now: `now == now` was "undefined variable: instant_at")
use crate::common::fails_with;
use crate::is;

#[test]
fn test_time_is_the_type_and_time_now_the_time_of_day() {
	is!("now is time", true);
	is!("type(time.now)", "time of day");
	is!("time.now is time", true);
	is!("time.now.hour == now.hour", true);
	is!("time = 3; time", 3);
}

#[test]
fn test_now_has_a_time_of_day_and_a_date() {
	is!("type(now.time)", "time of day");
	is!("type(now.date)", "date");
	is!("now.time.minute == now.minute", true);
	is!("now.date.year == now.year", true);
	is!("str(2026-10-14T22:27:46.5.time)", "22:27:46");
	is!("str(2026-10-14T22:27.time)", "22:27:00");
	is!("str(2026-10-14T22:27.date)", "2026-10-14");
	fails_with("2026-10-14.time", "date has no time of day");
	fails_with("now.time.year", "time of day has no year");
}

#[test]
fn test_a_time_of_day_goes_round_the_clock() {
	is!("str(2026-10-14T23:00.time + 2 hours)", "01:00:00");
	is!("later = now.time + 24 hours; later.hour == now.hour", true);
}

#[test]
fn test_clock_reads_compare() {
	is!("now <= now", true);
	is!("x = now; x <= now", true);
}
