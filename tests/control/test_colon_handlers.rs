// A handler's body after a colon, mid-program (card colon-body): `on every 5 seconds: print n` arrives as
// `on every (5 seconds: print) n`, the colon binding its neighbours; the words after it belong to the body
use crate::is;

#[test]
fn a_timer_body_after_a_colon_is_the_rest_of_the_line() {
	is!("n = 1\non every 5 seconds: print n\nn + 1", 2);
	is!("n = 1\non every 5 seconds: print n + 1\nn", 1);
	is!("n = 1\nat 9pm: print n\nn + 1", 2);
}

#[test]
fn a_listener_body_after_a_colon_is_the_rest_of_the_line() {
	is!("x = 0\ny = 0\nwhenever x > 1: y = x + 1\nx = 5\ny", 6);
	is!("x = 0\ny = 0\nwhenever x > 1: print x\nx = 5\nx", 5);
}
