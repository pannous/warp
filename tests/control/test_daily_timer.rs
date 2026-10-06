// `on every day at 9:00 {…}` (card g-3Gdo, notes/system_signals.md): a timer whose first tick is the next 9:00 local
// time, then every 24 hours; main runs on as usual
use crate::is;

#[test]
fn a_daily_timer_starts_and_main_runs_on() {
	is!("n = 1; on every day at 9:00 { n = 2 }; n", 1);
	is!("n = 1; on every day at 23:59 { n = 2 }; n + 1", 2);
}

#[test]
fn a_daily_timer_needs_a_time_of_day() {
	crate::common::fails_with("on every day at 25:00 { print 1 }", "a time of day");
	crate::common::fails_with("on every day at 9:75 { print 1 }", "a time of day");
}

#[test]
fn the_delay_until_a_time_of_day_wraps_to_tomorrow() {
	use warp_runtime::system_signals::seconds_until;
	assert_eq!(seconds_until(9 * 60, 8 * 3600), 3600);
	assert_eq!(seconds_until(9 * 60, 9 * 3600), 24 * 3600, "at 9:00 sharp the next tick is tomorrow's");
	assert_eq!(seconds_until(9 * 60, 10 * 3600), 23 * 3600);
	assert_eq!(seconds_until(0, 23 * 3600 + 59 * 60 + 59), 1);
}
