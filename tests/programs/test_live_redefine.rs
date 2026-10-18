// card live-redefine: a live loop defined again by its name (`live_loop drums { … }`) keeps playing as the same voice,
// with the new block from the first cycle that starts at or after the moment of the redefinition; the first one's
// count stays (none: endless). Dropped by decision music-plain-details: live loops are a plain library, live_loop(drums,
// 4), and the playground's handover between runs stays
#![cfg(feature = "native")]
use super::test_sound_voices::rendered;

const RATE: usize = 22050;

fn silent(samples: &[i16]) -> bool {
	samples.iter().all(|&sample| sample == 0)
}

#[test]
#[ignore = "in-program redefinition dropped (decision music-plain-details)"]
fn a_redefined_loop_plays_its_new_block_from_the_next_cycle() {
	let program = "use sound\ntempo = 120\nlive_loop drums 4 { pattern(\"~\", beat 1) }\nat beats 2 live_loop drums { pattern(\"c4\", beat 1) }\nawait drums\nsave sound as: \"SONG\"";
	let (seconds, samples) = rendered("redefined", program);
	assert_eq!(seconds, "2");
	assert!(silent(&samples[..RATE]), "the first two cycles keep the old, silent block");
	assert!(!silent(&samples[RATE..]), "the last two cycles play the new block");
}

#[test]
fn a_loop_defined_once_is_unchanged() {
	let program =
		"use sound\ntempo = 120\ndrums() := pattern(\"c4\", beat 1)\nvoice = live_loop(drums, 2)\nawait voice\nsave sound as: \"SONG\"";
	let (seconds, samples) = rendered("once", program);
	assert_eq!(seconds, "1");
	assert!(!silent(&samples));
}

// card playground-live: each cycle of a loop starts with live_cycle_started("drums") for the playground's handover;
// natively it does nothing
#[test]
fn a_named_loop_announcing_its_cycles_still_runs() {
	crate::is!("use sound\ndrums() := play(C4, 0.1s)\nvoice = live_loop(drums, 2)\nawait voice\n7", 7);
}
