// Times of day (card time-day, notes/system_signals.md): `at 9:00 {…}` runs once at the next 9:00 local time,
// `on every day at 9:30pm {…}` takes am/pm, `on every monday at 9:00 {…}` and `on every weekday at 7:15am {…}` only
// on those days; main runs on as usual
use crate::is;

fn lowered(code: &str) -> String {
	warp::pipeline::lower(code).expect("a program that needs a module").serialize()
}

#[test]
fn a_one_shot_time_starts_and_main_runs_on() {
	is!("n = 1; at 9:00 { n = 2 }; n", 1);
	is!("n = 1\nat 9pm { print n }\nn + 1", 2);
	assert!(lowered("at 9pm: print 1").contains("(signal_at 0 1260)"));
}

#[test]
fn am_and_pm_name_the_half_of_the_day() {
	assert!(lowered("on every day at 9:30pm { print 1 }").contains("(signal_daily 0 1290 127)"));
	assert!(lowered("on every day at 9:30 pm { print 1 }").contains("(signal_daily 0 1290 127)"));
	assert!(lowered("on every day at 12am { print 1 }").contains("(signal_daily 0 0 127)"));
	assert!(lowered("on every day at 12:15pm { print 1 }").contains("(signal_daily 0 735 127)"));
	assert!(lowered("at 7am { print 1 }").contains("(signal_at 0 420)"));
}

#[test]
fn weekdays_name_the_days_a_timer_runs() {
	assert!(lowered("on every monday at 9:00 { print 1 }").contains("(signal_daily 0 540 2)"));
	assert!(lowered("on every sunday at 9:00 { print 1 }").contains("(signal_daily 0 540 1)"));
	assert!(lowered("on every weekday at 7:15am { print 1 }").contains("(signal_daily 0 435 62)"));
	assert!(lowered("on every weekend at 10:00 { print 1 }").contains("(signal_daily 0 600 65)"));
	is!("n = 1; on every friday at 17:00 { n = 2 }; n", 1);
}

#[test]
fn a_time_of_day_must_be_one() {
	crate::common::fails_with("at 13:00pm { print 1 }", "a time of day");
	crate::common::fails_with("on every day at 0am { print 1 }", "a time of day");
	crate::common::fails_with("on every funday at 9:00 { print 1 }", "monday");
}

#[test]
#[cfg(feature = "native")]
fn the_delay_until_a_weekday_skips_the_other_days() {
	use warp_runtime::system_signals::seconds_until_on;
	const MONDAY: i64 = 1 << 1;
	const HOUR: i64 = 3600;
	// weekday 1 is Monday, 0 Sunday (as C's tm_wday)
	assert_eq!(seconds_until_on(9 * 60, MONDAY, 1, 8 * HOUR), HOUR);
	assert_eq!(seconds_until_on(9 * 60, MONDAY, 1, 9 * HOUR), 7 * 24 * HOUR, "at Monday 9:00 sharp the next is next week's");
	assert_eq!(seconds_until_on(9 * 60, MONDAY, 0, 10 * HOUR), 23 * HOUR, "Sunday 10:00 to Monday 9:00");
	assert_eq!(seconds_until_on(9 * 60, 0b0111110, 5, 10 * HOUR), (2 * 24 + 23) * HOUR, "Friday 10:00 to Monday 9:00");
}
