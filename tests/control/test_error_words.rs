//! card raise-boom: `fail` and `panic` raise as `raise` and `throw` do, `warn` warns as `warning` does; a program's
//! own definition of one of these words wins
use crate::common::fails_with;
use crate::is;

#[test]
fn fail_and_panic_raise() {
	fails_with("fail \"boom\"", "boom");
	fails_with("x = 1; panic \"stop\"; x", "stop");
	is!("try fail \"boom\" else 3", 3);
	is!("try { panic \"boom\" } catch e { e.message }", "boom");
}

#[test]
fn warn_warns_and_continues() {
	is!("warn \"careful\"; 1", 1);
	is!("\"a\" + warn(\"careful\") + \"b\"", "ab");
	// strict mode makes a warning the error it is
	is!("use strict; warn \"careful\"", warp::error("careful"));
}

#[test]
fn a_program_may_define_the_words() {
	is!("fail(x) := x + 1; fail 2", 3);
	is!("warn(x) := x * 2; warn 4", 8);
}
