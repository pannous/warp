// card sound-samples in the browser: a WAV the run wrote reads back as a sample (host-sound.js wavFrames), plays at a rate,
// and render_sound mixes it with synthesis at the higher rate (atRate); natively tests/programs/test_sound_samples.rs
#![cfg(not(feature = "native"))]
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_written_wav_is_a_sample_rendered_beside_a_tone() {
	is!(
		"use sound\ntone(440Hz, 0.5s)\nrender_sound(\"a.wav\")\nkick = sample(\"a.wav\")\nplay_sample(kick, 2)\ntone(440Hz, 0.5s)\n[kick.rate, kick.channels, round(render_sound(\"b.wav\") * 100)]",
		parse("[22050 1 75]")
	);
}
