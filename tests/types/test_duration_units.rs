//! Decision duration-units (card time-module): the time module's durations and the time units of lib/units.warp are one
//! thing, an amount of a time unit (`1 h`, `90 min`) is an exact duration wherever it meets a time or a duration
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_time_unit_adds_to_a_duration() {
	assert_eq!(shown("2 hour + 1 h"), "3 hours");
	assert_eq!(shown("1 day + 1 h"), "1 day 1 hour");
	assert_eq!(shown("2 hours - 30 min"), "1 hour 30 minutes");
	assert_eq!(shown("1 h + 2 hours"), "3 hours");
	assert_eq!(shown("d = 1 day; d + 1 h"), "1 day 1 hour");
}

#[test]
fn a_time_unit_compares_with_a_duration() {
	is!("1 h == 60 minutes", true);
	is!("2 hours > 1 h", true);
	is!("90 min < 1 day", true);
}

#[test]
fn a_time_unit_moves_a_time() {
	is!("2024-01-31T10:00 + 24 h == 2024-02-01T10:00", true);
	is!("(now + 1 h) > now", true);
}

#[test]
fn a_time_unit_alone_stays_a_quantity() {
	assert_eq!(shown("x = 3 h; x as minutes"), "180min");
	assert_eq!(shown("3 h * 2"), "6h");
	assert_eq!(shown("(2 hours as min) + 5 min"), "125min");
}
