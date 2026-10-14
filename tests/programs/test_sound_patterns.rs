// card patterns-live: `pattern "c4 e4 [g4 b4]*2 ~"` (Tidal's mini-notation) is one cycle of steps sharing the cycle's
// seconds, one sound; `live_loop 4 { … }` is a voice of its own repeating its block on the beat clock
// (lib/sound.warp parse_pattern, pattern_events; src/lowering/music_words.rs live_loop; samples/patterns.warp)
#![cfg(feature = "native")]
use super::test_sound_voices::rendered;

const RATE: usize = 22050;

fn silent(samples: &[i16]) -> bool {
	samples.iter().all(|&sample| sample == 0)
}

#[test]
fn a_cycle_lasts_its_seconds_whatever_its_steps() {
	for steps in ["c4 e4 g4 ~", "c4 [e4 g4]*3 ~", "c4(3,8)", "c4,e4,g4 ~ ~"] {
		let (seconds, samples) = rendered(
			"cycle",
			&format!("use sound\npattern \"{steps}\" for 1s\nrender_sound(\"SONG\")"),
		);
		assert_eq!(seconds, "1", "{steps}");
		assert_eq!(samples.len(), RATE, "{steps}");
	}
}

#[test]
fn a_bar_without_duration() {
	let (seconds, _) = rendered(
		"bar",
		"use sound\ntempo = 120\npattern \"c4 e4\"\nrender_sound(\"SONG\")",
	);
	assert_eq!(seconds, "2");
}

#[test]
fn a_rest_is_silent() {
	let (_, samples) = rendered("rest", "use sound\npattern \"~ c4\" for 1s\nrender_sound(\"SONG\")");
	assert!(silent(&samples[..RATE / 2 - 10]), "the rest");
	assert!(!silent(&samples[RATE / 2..]), "the note");
}

#[test]
fn euclid_spreads_three_notes_over_eight_steps() {
	let (_, samples) = rendered(
		"euclid",
		"use sound\npattern \"c4(3,8)\" for 1s\nrender_sound(\"SONG\")",
	);
	let sounding: Vec<bool> = (0..8)
		.map(|step| !silent(&samples[step * RATE / 8 + 50..(step + 1) * RATE / 8 - 50]))
		.collect();
	assert_eq!(sounding, [true, false, false, true, false, false, true, false]);
}

#[test]
fn swing_holds_the_first_note_longer() {
	let (_, straight) = rendered("straight", "use sound\npattern \"c4 ~\" for 1s\nrender_sound(\"SONG\")");
	let (_, swung) = rendered(
		"swung",
		"use sound\nswing = 1\npattern \"c4 ~\" for 1s\nrender_sound(\"SONG\")",
	);
	assert!(silent(&straight[RATE * 6 / 10..RATE * 9 / 10]));
	assert!(
		!silent(&swung[RATE * 6 / 10..RATE * 61 / 100]),
		"the note goes on past the half"
	);
}

#[test]
fn a_live_loop_is_a_voice_that_repeats() {
	let (seconds, samples) = rendered("loop", "use sound\ntempo = 120\nlive_loop 3 { pattern \"c4 e4\" for 1 beat }\nawait live_voice\nrender_sound(\"SONG\")");
	assert_eq!(seconds, "1.5");
	assert_eq!(samples.len(), RATE * 3 / 2);
}

#[test]
fn live_loops_play_along() {
	let program = "use sound\ntempo = 120\nlive_loop 2 { pattern \"c4 e4\" for 1 beat }\nplay G3 for 1 bar\nawait live_voice\nrender_sound(\"SONG\")";
	let (seconds, _) = rendered("along", program);
	assert_eq!(seconds, "2");
}

#[test]
fn pattern_samples_ends_each_event_at_its_cumulated_seconds() {
	let (seconds, samples) = rendered(
		"events",
		"use sound\nsound(pattern_samples([[[], 0.5], [[69], 0.5]]))\nrender_sound(\"SONG\")",
	);
	assert_eq!(seconds, "1");
	assert_eq!(samples.len(), RATE);
	assert!(silent(&samples[..RATE / 2 - 10]), "the rest");
	assert!(!silent(&samples[RATE / 2..]), "the note");
}
