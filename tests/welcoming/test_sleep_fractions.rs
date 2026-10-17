// A fraction of a second sleeps its milliseconds, to the nearest one: `sleep(1 / 60)` once handed the host the exact
// number 50/3's own encoding, which natively slept 0 ms and in the playground ended the run at once (its runaway
// deadline moved by -5.8e18 ms, card playground-tour: the shader samples' frame loops)
use crate::is;

#[test]
fn a_fraction_of_a_second_sleeps_its_milliseconds() {
	is!("start = clock(); sleep(1 / 60); clock() - start >= 16", true);
	is!("t = 1/30; start = clock(); sleep(t); clock() - start >= 33", true);
	is!("nap(t) := { sleep(t) }; start = clock(); nap(1/3); clock() - start >= 333", true);
}

#[test]
fn a_fraction_of_a_millisecond_rounds_to_the_nearest() {
	is!("start = clock(); sleep(16.7 ms); clock() - start >= 17", true);
	is!("start = clock(); sleep((50/3) ms); clock() - start >= 17", true);
}
