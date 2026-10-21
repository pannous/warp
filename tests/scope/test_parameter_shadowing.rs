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
linear left = float[512]
linear right = float[512]
strum(t) := for i in 1 to #left {
	left#i = 0.4 * sin(2 * pi * 262 * t)
	right#i = 0.4 * sin(2 * pi * 392 * t)
}
play_blocks(strum, [linear_address(left), linear_address(right)], 0.1s)
save sound as: \"SONG\"",
	);
	assert_eq!(seconds, "0.2");
	assert!(samples.iter().any(|&sample| sample != 0), "the signals sound");
}

/// card local-named-like-5: a local assigned a map in one function leaves a same-named parameter of another a list
/// (emitter.warp's entries = {} beside map_value(body, entries))
#[test]
fn a_parameter_is_no_map_another_function_assigned() {
	is!(
		"both(first: any): any := { held = {}; for name in first.keys { held[name] = first[name] }; held }\ndef firsts(held: any): list { held.map(e => e#1) }\nfirsts([[1, 2], [3, 4]])",
		warp::ints(vec![1, 3])
	);
	is!("f() := { held = {a: 1}; held.a }\ng(held: any) := held.map(e => e * 2)\ng([1, 2])", warp::ints(vec![2, 4]));
}

/// card local-named-like-5 after review: each function's variables are its own scope, its loop variables too
#[test]
fn a_loop_variable_is_no_map_another_function_assigned() {
	is!(
		"f() := { held = {a: 1}; held.a }\ng() := { n = []; for held in [[1, 2], [3]] { n += held.map(e => e * 2) }; n }\ng()",
		warp::ints(vec![2, 4, 6])
	);
}
