// card two-signal-kinds: a function's parameter is another variable than a same-named variable assigned in another
// function; the whole-program analyses of assigned values (walked numbers, exact-real types) took them for one
use crate::is;

#[test]
fn a_parameter_may_walk_what_another_function_assigned_a_number() {
	is!("f(x) := { pitch = 440; 0.3 }\ng(pitch) := [p for p in pitch]\ng([1, 2])", warp::ints(vec![1, 2]));
}

#[test]
fn a_parameter_is_not_typed_by_another_functions_real_value() {
	is!("f(t) := { pitch = pi * t; 0.3 }\nh(pitch) := if pitch is list then 1 else 2\nh(4.0)", 2);
}

#[cfg(feature = "native")]
#[test]
fn a_signal_and_a_block_play_in_one_program() {
	let (seconds, samples) = crate::programs::test_sound_voices::rendered(
		"two-signals",
		"use sound
siren(t) := {
	pitch = 440 + 110 * sin(pi * t)
	0.3 * sin(2 * pi * pitch * t)
}
play(siren, 0.1s)
chord(left, right, t) := for i in 1 to #left {
	left#i = 0.4 * sin(2 * pi * 262 * t)
	right#i = 0.4 * sin(2 * pi * 392 * t)
}
play(chord, 0.1s)
save sound as \"SONG\"",
	);
	assert_eq!(seconds, "0.2");
	assert!(samples.iter().any(|&sample| sample != 0), "the signals sound");
}
