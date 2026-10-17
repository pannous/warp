// card exact-div: an Int quotient that goes straight into a float is f64(a) / f64(b) when both are exact f64s (IEEE
// division rounds it once, as the exact quotient rounded is), with no exact quotient allocated per item; elsewhere
// `i / n` stays exact
use crate::common::fails_with;
use crate::is;

#[test]
fn an_int_quotient_into_a_float_is_rounded_once() {
	is!("xs = float[3]; n = 3 + random_below(1); for i in 1 to 3 { xs#i = i / n }; xs#1", 1.0 / 3.0);
	is!("n = 7 + random_below(1); float y = -n / 2; y", -3.5);
	is!("n = 10 + random_below(1); float y = n^20 / n^19; y", 10.0);
	// 2^53 + 1 is no f64: f64 division would round twice (…330.5), the exact quotient is 3002399751580331
	is!("n = 3 + random_below(1); float y = (2^53 + 1) / n; y", 3002399751580331.0);
}

#[test]
fn an_int_quotient_into_a_float_still_fails_dividing_by_zero() {
	fails_with("n = random_below(1); float y = 1 / n; y", "divide by zero");
	fails_with("xs = float[2]; n = random_below(1); xs#1 = 1 / n; xs#1", "divide by zero");
}

#[test]
fn an_int_quotient_elsewhere_stays_exact() {
	is!("n = 3 + random_below(1); x = 1 / n; x * 3", 1);
}
