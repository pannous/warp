// `^` decided at run time: an exact base with an exponent that may be a ratio (`2^(n/12)`, card unit-power) and a
// number whose kind is known only at run time (a list parameter's element, card run-time-power) give the exact power
// when the exponent is whole, else the f64 one; they failed with "an exact power needs an integer exponent" or
// "not an int"
use crate::is;

#[test]
fn a_run_time_ratio_exponent_is_a_float_power() {
	is!("n = 7; round(2^(n/12) * 1000)", 1498);
	is!("semitones(n) := 2^(n/12); round(440 * semitones(7))", 659);
	is!("print 2km; n = 7; round(2^(n/12) * 1000)", 1498);
	is!("n = 24; 2^(n/12)", 4);
	is!("n = -12; 2^(n/12)", 0.5);
}

#[test]
fn a_run_time_kind_has_a_power() {
	is!("pick(xs) := { f = xs#1; f ^ 2 }; round(pick([sqrt(2)]) * 1000)", 2000);
	is!("pick(xs) := { f = xs#1; f ^ 3 }; pick([2])", 8);
	is!("pick(xs) := { f = xs#1; 2 ^ f }; pick([0.5]) > 1.41", true);
}

/// a float context reads a run-time-kind result of a call as its f64 (it read it as an Int: "not an int")
#[test]
fn a_run_time_kind_call_is_rounded() {
	is!("pick(xs) := { f = xs#1; f * 1 }; round(pick([sqrt(2)]) * 1000)", 1414);
	is!("pick(xs) := { f = xs#1; f * 1 }; round(pick([7]))", 7);
}

/// ‖x‖ of a run-time-kind number: a Float's is a Float, an Int's stays exact (it read the Float as an Int)
#[test]
fn a_run_time_kind_has_an_absolute_value() {
	is!("ratio(k) := 2^((k - 69) / 12); round(abs(ratio(57)) * 1000)", 500);
	is!("ratio(k) := 2^((k - 69) / 12); abs(-ratio(40)) < 0.19", true);
	is!("ratio(k) := 2^((k - 69) / 12); abs(ratio(81))", 2);
}

// card ratio-exponent: a decimal exponent is a float (decision exact-default), so the power is the f64 one; it failed
// with "an exact power needs an integer exponent"
#[test]
fn a_decimal_exponent_is_a_float_power() {
	is!("l = -0.45; round(10^l * 1000)", 355);
	is!("l = -0.45; type(10^l)", "float");
	is!("x = 2.5; 4^x", 32.0);
}
