// card stereo-pan in the browser: a panned sound is stereo, its frames counted as such on the voice and in the render
// (host.js sound_samples, host-files.js mixed); natively tests/programs/test_sound_pan.rs
#![cfg(not(feature = "native"))]
use crate::is;

#[test]
fn mono_and_stereo_render_as_long_as_they_sound() {
	is!("use sound\nplay(C4, 0.1s)\npan = 1.0\nplay(E4, 0.1s)\nround(render_sound(\"pan.wav\") * 10)", 2);
}
