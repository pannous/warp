//! `now` compared with a part of a time compares that part of now (cards now-make, now-allow): with a duration its local
//! time of day (`now > 17h`), with a month name its month (`now >= October`), with a date its local date
//! (`now ≈ 2026-10-14`, `now > 2026-10-14`)
use crate::is;

#[test]
fn test_now_against_a_time_of_day() {
	is!("now < 24h", true);
	is!("now >= 0h", true);
	is!("(now > 17h) == (time > 17h)", true);
}

#[test]
fn test_now_against_a_month() {
	is!("now >= January", true);
	is!("now <= December", true);
	is!("(now >= October) == (now.month >= 10)", true);
	is!("now ≈ January or now ≈ February or now.month > 2", true);
}

#[test]
fn test_now_against_a_date() {
	is!("now ≈ 1970-01-01", false);
	is!("now > 1970-01-01", true);
	is!("now < 2999-01-01", true);
	is!("2999-01-01 > now", true);
}

#[test]
fn test_own_names_stay() {
	is!("October = 3; October + 1", 4);
}
