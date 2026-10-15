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
		let (seconds, samples) =
			rendered("cycle", &format!("use sound\npattern(\"{steps}\", 1s)\nsave sound as \"SONG\""));
		assert_eq!(seconds, "1", "{steps}");
		assert_eq!(samples.len(), RATE, "{steps}");
	}
}

#[test]
fn a_bar_without_duration() {
	let (seconds, _) = rendered("bar", "use sound\ntempo = 120\npattern \"c4 e4\"\nsave sound as \"SONG\"");
	assert_eq!(seconds, "2");
}

#[test]
fn a_rest_is_silent() {
	let (_, samples) = rendered("rest", "use sound\npattern(\"~ c4\", 1s)\nsave sound as \"SONG\"");
	assert!(silent(&samples[..RATE / 2 - 10]), "the rest");
	assert!(!silent(&samples[RATE / 2..]), "the note");
}

#[test]
fn euclid_spreads_three_notes_over_eight_steps() {
	let (_, samples) = rendered("euclid", "use sound\npattern(\"c4(3,8)\", 1s)\nsave sound as \"SONG\"");
	let sounding: Vec<bool> =
		(0..8).map(|step| !silent(&samples[step * RATE / 8 + 50..(step + 1) * RATE / 8 - 50])).collect();
	assert_eq!(sounding, [true, false, false, true, false, false, true, false]);
}

#[test]
fn swing_holds_the_first_note_longer() {
	let (_, straight) = rendered("straight", "use sound\npattern(\"c4 ~\", 1s)\nsave sound as \"SONG\"");
	let (_, swung) = rendered("swung", "use sound\nswing = 1\npattern(\"c4 ~\", 1s)\nsave sound as \"SONG\"");
	assert!(silent(&straight[RATE * 6 / 10..RATE * 9 / 10]));
	assert!(!silent(&swung[RATE * 6 / 10..RATE * 61 / 100]), "the note goes on past the half");
}

#[test]
fn a_live_loop_is_a_voice_that_repeats() {
	let (seconds, samples) = rendered(
		"loop",
		"use sound\ntempo = 120\nlive_loop 3 { pattern(\"c4 e4\", 1 beat) }\nawait live_voice\nsave sound as \"SONG\"",
	);
	assert_eq!(seconds, "1.5");
	assert_eq!(samples.len(), RATE * 3 / 2);
}

#[test]
fn live_loops_play_along() {
	let program = "use sound\ntempo = 120\nlive_loop 2 { pattern(\"c4 e4\", 1 beat) }\nplay(G3, 1 bar)\nawait live_voice\nsave sound as \"SONG\"";
	let (seconds, _) = rendered("along", program);
	assert_eq!(seconds, "2");
}

#[test]
fn pattern_samples_ends_each_event_at_its_cumulated_seconds() {
	let (seconds, samples) =
		rendered("events", "use sound\nsound(pattern_samples([[[], 0.5], [[69], 0.5]]))\nsave sound as \"SONG\"");
	assert_eq!(seconds, "1");
	assert_eq!(samples.len(), RATE);
	assert!(silent(&samples[..RATE / 2 - 10]), "the rest");
	assert!(!silent(&samples[RATE / 2..]), "the note");
}

// card pattern-extras: `<a b>` alternation, `_` / `@n` elongation, `?` seeded chance, named live loops
fn halves_sounding(program: &str, name: &str, cycles: usize) -> Vec<bool> {
	let (_, samples) = rendered(name, program);
	(0..cycles * 2).map(|half| !silent(&samples[half * RATE / 4 + 50..(half + 1) * RATE / 4 - 50])).collect()
}

#[test]
fn alternation_takes_one_per_cycle_in_turn() {
	let program = "use sound\nfor i in 0..4 { pattern(\"c4 <e4 ~ g4>\", 0.5s) }\nsave sound as \"SONG\"";
	assert_eq!(halves_sounding(program, "alternation", 4), [true, true, true, false, true, true, true, true]);
}

#[test]
fn nested_alternation_turns_on_its_own_visits() {
	let program = "use sound\nfor i in 0..4 { pattern(\"<c4 <~ e4>>\", 0.25s) }\nsave sound as \"SONG\"";
	let (_, samples) = rendered("nested", program);
	let sounding: Vec<bool> =
		(0..4).map(|cycle| !silent(&samples[cycle * RATE / 4 + 50..(cycle + 1) * RATE / 4 - 50])).collect();
	assert_eq!(sounding, [true, false, true, true]);
}

#[test]
fn elongation_holds_a_step_longer() {
	for steps in ["c4 _ ~", "c4@2 ~"] {
		let (_, samples) =
			rendered("elongated", &format!("use sound\npattern(\"{steps}\", 1.5s)\nsave sound as \"SONG\""));
		assert_eq!(samples.len(), RATE * 3 / 2, "{steps}");
		assert!(!silent(&samples[RATE * 9 / 10..RATE * 95 / 100]), "{steps}: c4 sounds into its second step");
		assert!(silent(&samples[RATE * 105 / 100..]), "{steps}: then the rest");
	}
}

#[test]
fn chance_drops_some_cycles_the_same_each_run() {
	let program = "use sound\nfor i in 0..8 { pattern(\"c4? c4\", 0.5s) }\nsave sound as \"SONG\"";
	let first = halves_sounding(program, "chance", 8);
	assert_eq!(first, halves_sounding(program, "chance-again", 8), "seeded: the same each run");
	let maybe: Vec<bool> = first.iter().step_by(2).copied().collect();
	assert!(maybe.contains(&true) && maybe.contains(&false), "{maybe:?}");
	assert!(first.iter().skip(1).step_by(2).all(|&sounds| sounds), "c4 without ? always sounds");
	let reseeded = halves_sounding(&program.replace("use sound\n", "use sound\npattern_seed = 7\n"), "reseeded", 8);
	assert_ne!(first, reseeded, "another seed, other cycles");
}

#[test]
fn named_live_loops_are_awaited_each() {
	let program = "use sound\ntempo = 120\nlive_loop bass 2 { pattern(\"c3\", 1 beat) }\nlive_loop lead 3 { pattern(\"e4\", 1 beat) }\nawait bass\nawait lead\nsave sound as \"SONG\"";
	let (seconds, _) = rendered("named", program);
	assert_eq!(seconds, "1.5");
}
