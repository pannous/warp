//! The standard library module time (notes/stdlib.md): the calendar over clock()
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn dates_of_milliseconds() {
	is!("use time; d = date_of(0); [d.year, d.month, d.day]", parse("[1970 1 1]"));
	is!("use time; d = date_of(951782400000); [d.year, d.month, d.day]", parse("[2000 2 29]"));
	is!("use time; weekday(0)", 4);
}

#[test]
fn today_is_a_date_after_2025() {
	is!("use time; today().year > 2025", 1);
}

#[test]
fn date_arithmetic() {
	is!("use time; d = date_of(add_days(0, 31)); [d.month, d.day]", parse("[2 1]"));
	is!("use time; days_between(0, 951782400000)", 11016);
}

#[test]
fn use_time_formats_dates_and_times() {
	is!("use time; format_date(0)", "1970-01-01");
	is!("use time; format_date(1791331509000)", "2026-10-07");
	is!("use time; format_time(1791331509000)", "00:05:09");
	is!("use time; two_digits(7) + two_digits(12)", "0712");
}
