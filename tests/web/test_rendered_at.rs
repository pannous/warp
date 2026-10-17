// card sample-accurate in the browser: `at` places sounds on the voice and render_sound keeps the silence before
// (host-tasks.js voiceAt, host-files.js mixed); natively tests/programs/test_sound_at.rs
#![cfg(not(feature = "native"))]
use crate::is;

#[test]
fn a_render_keeps_the_silence_before_an_at() {
	is!("use sound\nat 1s beep()\nplay(C4, 0.1s)\nround(save(sound, \"at.wav\") * 100)", 115);
}
