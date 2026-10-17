// card real-time in the browser: play of a function of the time, of a block function, of a pair in stereo computes its
// samples in the program's Worker (host-files.js STD_ADAPTERS.sound.signal, blocks) and plays them as one sound, the
// render as long as natively (tests/programs/test_sound_signal.rs)
#![cfg(not(feature = "native"))]
use crate::is;

#[test]
fn a_signal_function_plays_in_the_browser() {
	is!(
		"use sound\nhum(t) := 0.5 * sin(2 * pi * 441 * t)\nplay(hum, 0.1s)\nround(save(sound, \"signal.wav\") * 100)",
		10
	);
	is!("use sound\nleft(t) := 0.5 * sin(2 * pi * 441 * t)\nright(t) := 0.25 * t\nplay([left, right], 0.1s)\nround(save(sound, \"stereo.wav\") * 100)", 10);
}

#[test]
fn a_block_function_plays_in_the_browser() {
	is!("use sound\nglobal calls = 0\nhum(block, t) := {\n  calls = calls + 1\n  for i in 1 to #block { block#i = 0.5 * sin(2 * pi * 441 * (t + (i - 1) / sample_rate)) }\n}\nplay(hum, 0.1s)\nround(save(sound, \"blocks.wav\") * 100) + calls", 15);
	is!("use sound\nhum(left, right, t) := for i in 1 to #left { left#i = 0.1; right#i = 0.2 }\nplay(hum, 0.1s)\nround(save(sound, \"blocks_stereo.wav\") * 100)", 10);
}
