//! A main-level list of the program's float-returning calls is a list of floats inside functions too
//! (card float-calls: found rewriting samples/neural_net.wasp)
use crate::is;

const HALF: &str = "w() := 0.5 as float; ";

#[test]
fn a_list_of_float_calls_is_read_as_floats_in_a_function() {
	is!(&format!("{HALF}xs = [w(), w()]; f(j) := {{ t = xs[j]; t + 1 }}; f(1)"), 1.5);
	is!(&format!("{HALF}xs = [w(), 0.25 as float]; f(j) := xs#j + 1; f(2)"), 1.25);
	is!("w() := random() - 1; xs = [w(), w()]; f(j) := { t = xs[j]; t + 1 }; f(1) < 1", true);
}
