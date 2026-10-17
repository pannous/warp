// card sound-gain-ramps in the browser: fade_sound(handle, gain, seconds) ramps the page's sound (playground.js faded,
// a GainNode per sound) and render_sound's mix (host-files.js fadedGain); natively tests/programs/test_sound_fades.rs
#![cfg(not(feature = "native"))]
use crate::is;

#[test]
fn fade_sound_runs_in_the_browser() {
	is!("use sound\na = tone(440Hz, 0.1s)\nb = tone(440Hz, 0.1s)\nfade_sound(a, 0, 0.05)\nfade_sound(0, 0.5)\nround(save(sound, \"fade.wav\") * 10)", 2);
}
