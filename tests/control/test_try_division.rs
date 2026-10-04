// `try a / b else c` checks the divisor before dividing and catches any other error of the division too (a text
// divided was an uncaught "not a number")
use warp::is;

#[test]
fn try_catches_every_error_of_a_division() {
	is!("x = 5; try x / 0 else -1", -1);
	is!("xs = [1, \"a\"]; try xs#2 / 2 else -1", -1);
	is!("xs = [1.5, \"a\"]; try xs#2 / 2.0 else -1", -1);
	is!("x = 9; try x / 3 else -1", 3);
}
