//! A bare `time` is the type, `time.now` the moment (card print-time, user decision); two clock reads compare
//! (card time-now: `now == now` was "undefined variable: instant_at")
use crate::is;

#[test]
fn test_time_is_the_type_and_time_now_the_moment() {
	is!("now is time", true);
	is!("type(time.now)", "instant");
	is!("time.now is instant", true);
	is!("time = 3; time", 3);
}

#[test]
fn test_clock_reads_compare() {
	is!("now == now", true);
	is!("now != now", false);
	is!("x = now; x == now", true);
}
