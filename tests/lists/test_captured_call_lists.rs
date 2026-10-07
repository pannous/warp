// card float-calls: a list of calls of a function returning floats, read inside another function, holds floats:
// the function's own analysis knew the calls' kinds only at the top level
use crate::is;

#[test]
fn a_captured_list_of_float_calls_holds_floats() {
	is!("w() := random() * 0.0 + 0.25; xs = [w(), w()]; f(j) := { t = xs[j]; t + 1 }; f(1)", 1.25);
	is!("def w() { return random() * 0.0 + 0.5 }; xs = [w(), w()]; f(j) := { xs[j] * 2 }; f(0)", 1.0);
}
