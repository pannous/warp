// card time-day-value (the user's example `if time < 24h : print ("There's still some time left today.")`): `time` is
// the local time of day, a duration since midnight (as `on every day at 9am` reads the clock), compared with a duration;
// a comparison of constant quantities is decided when compiled, so it works in a condition and with a variable
use crate::common::fails_with;
use crate::is;

#[test]
fn time_is_the_time_of_day() {
	is!("if time < 24h : \"still today\"", "still today");
	is!("if time < 0 h : \"never\" else \"always\"", "always");
	is!("time >= 0 ms and time < 1 day", 1);
	fails_with("time < 2 m", "DimensionError");
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
