// Card destructured-float (samples/neural_net.wasp): a float a tuple function gives back with `return a, b` stays a
// float through a destructuring statement, a call taking it and a float global a function changes
use crate::is;

const SIGMOID_PAIR: &str = "def sigmoid(x) := 1.0 / (1.0 + exp(-x)); def pair(x) { return [1], sigmoid(x) }";

#[test]
fn a_destructured_float_stays_a_float() {
	is!(&format!("{SIGMOID_PAIR}; def t() {{ (h, o) = pair(0); o }}; t()"), 0.5);
	is!(&format!("{SIGMOID_PAIR}; def t() {{ (h, o) = pair(0); b = 0.25; b = b + o }}; t()"), 0.75);
	is!(&format!("{SIGMOID_PAIR}; def slope(x) := x * (1.0 - x); def t() {{ (h, o) = pair(0); slope(o) }}; t()"), 0.25);
}

#[test]
fn a_function_changes_a_float_global() {
	is!("b = random(); def bump() { global b; b = b + 0.5; b }; bump(); b >= 0.5", true);
	is!("b = random(); def bump() { global b; b += 0.5; 1 }; bump(); b >= 0.5", true);
	is!(&format!("{SIGMOID_PAIR}; b = sigmoid(0); def t() {{ global b; (h, o) = pair(0); b = b + o }}; t(); b"), 1.0);
}

/// a value held as a Node (a parameter fed only loop items, `for (i, t) in examples`) added into a float variable keeps
/// its float: it was read as an Int ('not an int')
#[test]
fn a_held_value_adds_into_a_float() {
	let loop_calls = "ex = [([0, 0], 0)]; for (i, t) in ex { f(t) }";
	is!(&format!("b = random(); def f(t) {{ global b; d = t - random(); b = b + 0.5 * d }}; {loop_calls}; b < 3"), true);
	is!(&format!("def f(t) {{ b = 0.25 + random() * 0; d = t + 0.5 + random() * 0; b = b + d; print b }}; {loop_calls}"), 1);
}
