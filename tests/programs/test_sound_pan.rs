// card stereo-pan: `pan` places the notes between the left (-1) and the right (1) speaker; a panned sound is stereo,
// its samples interleaved left, right (lib/sound.warp panned, sound_samples' channels), and render_sound mixes mono and
// stereo sounds into one stereo WAV
#![cfg(feature = "native")]
use super::test_sound_voices::rendered;
use crate::is;

const RATE: usize = 22050;

#[test]
fn panning_interleaves_the_channels() {
	is!("use sound\npan = 1.0\npanned([32868, 32668]) == [32768, 32868, 32768, 32668]", true);
	is!("use sound\npan = -0.5\npanned([32868]) == [32868, 32818]", true);
	is!("use sound\npanned([32868]) == [32868]", true); // in the middle a sound stays mono
}

#[test]
fn a_panned_note_sounds_on_one_side() {
	let (seconds, samples) = rendered("left", "use sound\npan = -1.0\nplay(C4, 0.1s)\nrender_sound(\"SONG\")");
	assert_eq!(seconds, "0.1");
	assert_eq!(samples.len(), 2 * RATE / 10);
	assert!(samples.iter().step_by(2).any(|&left| left != 0), "the left speaker sounds");
	assert!(samples.iter().skip(1).step_by(2).all(|&right| right == 0), "the right one is silent");
}

#[test]
fn mono_and_stereo_mix_into_stereo() {
	let (seconds, samples) =
		rendered("both", "use sound\nplay(C4, 0.1s)\npan = 1.0\nplay(E4, 0.1s)\nrender_sound(\"SONG\")");
	assert_eq!(seconds, "0.2");
	assert_eq!(samples.len(), 2 * RATE / 5);
	let (mono, panned) = samples.split_at(samples.len() / 2);
	assert!(mono.chunks(2).all(|frame| frame[0] == frame[1]), "the mono note sounds on both speakers alike");
	assert!(panned.chunks(2).all(|frame| frame[0] == 0), "the panned one on the right alone");
}
