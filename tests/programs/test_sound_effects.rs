// card signal-graph: every sound passes distortion → delay → reverb (module settings) between the voices and the output
#![cfg(feature = "native")]
use crate::is;

const TONE: &str = "use sound\nplain = chord_samples([440], 0.1, \"sine\")\n";

#[test]
fn a_delay_adds_a_tail_of_echoes() {
	is!(
		&format!(
			"{TONE}delay_time = 0.1\ndelay_feedback = 0.5\ncount(through_effects(plain)) > count(plain) + 2 * 2205"
		),
		true
	);
	is!(&format!("{TONE}count(through_effects(plain)) == count(plain)"), true);
}

#[test]
fn an_echo_sounds_where_the_dry_sound_was_silent() {
	// 0.1 s of tone, then 0.1 s gap: the echo sounds from 0.1 s on
	is!(&format!("{TONE}delay_time = 0.15\nmixed = through_effects(plain)\nmax([abs(x - sample_offset) for x in mixed[3300..4400]]) > 1000"), true);
}

#[test]
fn a_reverb_rings_after_the_end() {
	is!(&format!("{TONE}reverb_mix = 0.5\nmixed = through_effects(plain)\ncount(mixed) > count(plain) + 10000 and max([abs(x - sample_offset) for x in mixed[2300..2500]]) > 100"), true);
}

#[test]
fn distortion_flattens_the_peaks_and_stays_in_range() {
	is!(&format!("{TONE}distortion = 1.0\nmixed = through_effects(plain)\nmax(mixed) - sample_offset <= loudness * loudest_sample"), true);
	is!(&format!("{TONE}distortion = 1.0\nsum([abs(x - sample_offset) for x in through_effects(plain)]) > sum([abs(x - sample_offset) for x in plain])"), true);
}

#[test]
fn effects_play_in_a_program() {
	is!("delay_time = 1/4 beat\nreverb_mix = 0.2\ndistortion = 0.3\nplay(C4, 50ms)\n1", 1);
}
