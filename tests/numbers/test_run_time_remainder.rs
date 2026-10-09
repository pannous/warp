// `%` and `rem` of a number whose kind is known only at run time (an element of a list parameter) decide Int or Float
// by the values, like + - * / do (card float-modulo-element: a float element failed with "not an int")
use crate::common::fails_with;
use crate::is;

#[test]
fn a_float_element_has_a_float_remainder() {
	is!("phases(xs) := [f % 1 for f in xs]; phases([sqrt(2)])#1 > 0.41", true);
	is!("first(xs) := { f = xs#1; f % 1.5 }; first([sqrt(2) + 2])", 0.4142135623730949);
	is!("first(xs) := { f = xs#1; 7 % f }; first([2.5])", 2.0);
	is!("first(xs) := { f = xs#1; f rem 1 }; first([-1.25])", -0.25);
	is!("first(xs) := { f = xs#1; f % 1 }; first([-1.25])", 0.75);
}

#[test]
fn an_int_element_keeps_an_exact_remainder() {
	is!("first(xs) := { f = xs#1; f % 3 }; first([-7])", 2);
	is!("first(xs) := { f = xs#1; f rem 3 }; first([-7])", -1);
	fails_with("first(xs) := { f = xs#1; f % 0 }; first([7])", "divide by zero");
}

/// card long-float: a 17-digit literal is a float (a shorter one stays an exact decimal), so lib/sound.warp's note()
/// with the ratio 1.0594630943592953 handed chord_samples a float element (probes/long_float_note.warp)
#[test]
fn a_long_float_literal_element_has_a_remainder() {
	is!("ratio = 1.0594630943592953; phases(xs) := [f % 1 for f in xs]; round(phases([440.0 * ratio])#1 * 1000)", 164);
}
