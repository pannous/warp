// card sample-accurate: `at 2 beats play C4`, `at 1s { … }` places a statement's sounds at that time on the voice,
// counted from its first sound, and the voice then goes on where it stood; render_sound mixes them there to the sample
// and keeps the silence before (src/lowering/music_words.rs → lib/sound.warp sound_at, sound_at_end; src/sound.rs at)
#![cfg(feature = "native")]
use super::test_sound_voices::rendered;

const RATE: usize = 22050;

#[test]
fn a_statement_sounds_inside_another() {
	let (seconds, samples) = rendered("inside", "play C4 for 0.5s\nat 0.1s play E4 for 0.2s\nrender_sound(\"SONG\")");
	assert_eq!(seconds, "0.5");
	assert_eq!(samples.len(), RATE / 2);
}

#[test]
fn beats_place_a_block() {
	let program = "tempo = 120\nat 2 beats play C4 for 1 beat\nat 0 beats { play E4 for 0.25s\nplay G4 for 0.25s }\nrender_sound(\"SONG\")";
	let (seconds, samples) = rendered("beats", program);
	assert_eq!(seconds, "1.5");
	assert!(samples[RATE * 6 / 10..RATE * 9 / 10].iter().all(|&sample| sample == 0), "silent between the block and the note");
}

#[test]
fn the_silence_before_stays_and_the_voice_goes_on() {
	let (seconds, samples) = rendered("before", "at 1s beep()\nplay C4 for 0.1s\nrender_sound(\"SONG\")");
	assert!(seconds.starts_with("1.15"), "{seconds}"); // the beep's 0.15 s to the whole sample
	assert!(samples[..RATE / 10].iter().any(|&sample| sample != 0), "C4 at the start, where the voice stood");
	assert!(samples[RATE / 5..RATE].iter().all(|&sample| sample == 0), "nothing until the beep");
}

#[test]
fn the_words_it_lowers_to() {
	crate::is!("use sound\nsound_at(1)\nsound_at_end()\n7", 7);
}
