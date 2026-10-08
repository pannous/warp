//! The standard library module time (notes/stdlib.md): the calendar over clock()
use crate::is;
use warp::warp_parser::parse;

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

#[test]
fn use_time_parses_dates() {
	is!("use time; parse_date(\"1970-01-01\")", 0);
	is!("use time; parse_date(\"2026-10-07\")", 1791331200000i64);
	is!("use time; format_date(parse_date(\"2000-02-29\"))", "2000-02-29");
	is!("use time; days_between(parse_date(\"2026-01-01\"), parse_date(\"2026-12-25\"))", 358);
}

#[test]
fn format_duration_in_the_largest_units() {
	is!("use time; format_duration(5430000)", "1h 30m 30s");
	is!("use time; format_duration(250)", "250ms");
	is!("use time; format_duration(86400000 + 5000)", "1d 5s");
}

#[test]
fn use_time_knows_leap_years_and_month_lengths() {
	is!("use time; [is_leap_year(2024), is_leap_year(1900), is_leap_year(2000)]", warp::ints(vec![1, 0, 1]));
	is!("use time; [days_in_month(2024, 2), days_in_month(2023, 2), days_in_month(2024, 4), days_in_month(2024, 12)]", warp::ints(vec![29, 28, 30, 31]));
}
