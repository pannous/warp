// card sample-accurate in the browser: `at` places sounds on the voice and render_sound keeps the silence before
// (host-tasks.js voiceAt, host-files.js mixed); natively tests/programs/test_sound_at.rs
#![cfg(not(feature = "native"))]
use crate::is;

#[test]
fn a_render_keeps_the_silence_before_an_at() {
	is!("at 1s beep()\nplay C4 for 0.1s\nround(render_sound(\"at.wav\") * 100)", 115);
}
