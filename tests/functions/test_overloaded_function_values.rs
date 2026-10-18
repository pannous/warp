//! card program-function: a function value whose name is overloaded by arity, the program's `chord(t)` beside sound's
//! `chord(root, kind)`, is the variant a function-typed parameter takes, also through a parameter passed on to one
//! (src/lowering/definitions/overloads.rs passed_variants)
use crate::is;

const APPLY: &str = "apply(f: (float) -> float, x: float) := f(x)\n";

#[test]
fn an_overloaded_function_passed_for_a_function_typed_parameter_is_the_fitting_variant() {
	is!(&format!("{APPLY}half(a, b) := a + b\nhalf(t) := t / 2\napply(half, 3.0)"), 1.5);
	is!(&format!("{APPLY}outer(g, x) := apply(g, x)\nhalf(a, b) := a + b\nhalf(t) := t / 2\nouter(half, 3.0)"), 1.5);
}

#[test]
fn a_signal_named_like_a_sound_word_plays() {
	is!(
		"use sound\nchord(t) := 0.1 * sin(2 * pi * 441 * t)\nplay(chord, 0.1s)\nsave(sound, \"chord_signal.wav\") > 0",
		true
	);
	let blocks =
		"use sound\nlinear block = float[512]\nchord(t) := {\n  for i in 1 to #block { block#i = 0.1 }\n  0\n}\n";
	is!(
		&format!(
			"{blocks}play_blocks(chord, linear_address(block), 0.1s)\nround(save(sound, \"chord_blocks.wav\") * 100)"
		),
		10
	);
}
