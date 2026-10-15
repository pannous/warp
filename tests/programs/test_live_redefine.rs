// card live-redefine: a live loop defined again by its name (`live_loop drums { … }`) keeps playing as the same voice,
// with the new block from the first cycle that starts at or after the moment of the redefinition; the first one's
// count stays (none: endless). src/lowering/words/music_words.rs live_loop_statements, lib/sound.warp sound_moment
#![cfg(feature = "native")]
use super::test_sound_voices::rendered;

const RATE: usize = 22050;

fn silent(samples: &[i16]) -> bool {
	samples.iter().all(|&sample| sample == 0)
}

#[test]
fn a_redefined_loop_plays_its_new_block_from_the_next_cycle() {
	let program = "use sound\ntempo = 120\nlive_loop drums 4 { pattern(\"~\", 1 beat) }\nat 2 beats live_loop drums { pattern(\"c4\", 1 beat) }\nawait drums\nsave sound as \"SONG\"";
	let (seconds, samples) = rendered("redefined", program);
	assert_eq!(seconds, "2");
	assert!(silent(&samples[..RATE]), "the first two cycles keep the old, silent block");
	assert!(!silent(&samples[RATE..]), "the last two cycles play the new block");
}

#[test]
fn a_loop_defined_once_is_unchanged() {
	let program =
		"use sound\ntempo = 120\nlive_loop drums 2 { pattern(\"c4\", 1 beat) }\nawait drums\nsave sound as \"SONG\"";
	let (seconds, samples) = rendered("once", program);
	assert_eq!(seconds, "1");
	assert!(!silent(&samples));
}

// card playground-live: a named loop's block starts with live_cycle_started("drums") for the playground's handover;
// natively it does nothing
#[test]
fn a_named_loop_announcing_its_cycles_still_runs() {
	crate::is!("use sound\nlive_loop drums 2 { play(C4, 0.1s) }\nawait drums\n7", 7);
}
