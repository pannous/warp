// card time-day-value (the user's example `if time < 24h : print ("There's still some time left today.")`): `now`
// compared with a duration is the local time of day, a duration since midnight (as `on every day at 9am` reads the
// clock); a comparison of constant quantities is decided when compiled, so it works in a condition and with a variable.
// A bare `time` is the type (user decision print-time, which changed these from `time` to `now`)
use crate::common::fails_with;
use crate::is;

#[test]
fn time_is_the_time_of_day() {
	is!("if now < 24h : \"still today\"", "still today");
	is!("if now < 0 h : \"never\" else \"always\"", "always");
	is!("now >= 0 ms and now < 1 day", 1);
	fails_with("now < 2 m", "DimensionError");
	fails_with("time < 24h", "now < 24h");
}

#[test]
fn a_program_naming_time_keeps_its_own() {
	is!("time = 20h; if time < 24h : \"evening\" else \"night\"", "evening");
	is!("time = 3; time + 1", 4);
}

#[test]
fn constant_quantities_compare_in_a_condition() {
	is!("if 20h < 24h : \"evening\"", "evening");
	is!("x = 2 km; if x > 1500 m : \"far\" else \"near\"", "far");
}
