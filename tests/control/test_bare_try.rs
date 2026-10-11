// card failed-raised (user, option A): a raised error (1/0, raise, xs#5) still stops the program; a bare `try X`, no
// else, keeps it as the value, which `r failed` tests
use crate::common::fails_with;
use crate::is;

#[test]
fn a_bare_try_keeps_the_raised_error_as_its_value() {
	is!("r = try 10/0; if r failed then 1 else 2", 1);
	is!("r = try raise \"boom\"; if r failed then 1 else 2", 1);
	is!("xs = [1]; r = try xs#5; if r failed then 1 else 2", 1);
	is!("r = try 10/2; if r failed then 1 else r", 5);
}

#[test]
fn a_raise_without_try_still_stops_the_program() {
	fails_with("r = 10/0; if r failed then 1 else 2", "divide by zero");
}
