// card synth-filters (sound layer 3): module settings shape every note. `filter` ("lowpass", "highpass", "bandpass")
// at `cutoff` Hz with `resonance` (Q), `vibrato` (semitones) at `vibrato_rate` Hz, `tremolo` (a share of the gain) at
// `tremolo_rate` Hz, `unison` voices spread over `detune` semitones
#![cfg(feature = "native")]
use crate::is;

/// how loud a 4 kHz tone comes out after the settings, compared with the plain one: the loudest sample's share
fn high_tone_share(settings: &str) -> String {
	format!("use sound\nloudest() := max(chord_samples([4000], 0.1, \"sine\")) - sample_offset\nplain = loudest()\n{settings}\nloudest() / plain")
}

#[test]
fn a_lowpass_filter_quiets_high_notes() {
	is!(&format!("{} < 0.1", high_tone_share("filter = \"lowpass\"\ncutoff = 500")), true);
	is!(&format!("{} > 0.9", high_tone_share("filter = \"highpass\"\ncutoff = 500")), true);
	is!(&format!("{} > 0.9", high_tone_share("filter = \"bandpass\"\ncutoff = 4000\nresonance = 2.0")), true);
}

#[test]
fn tremolo_swells_the_gain() {
	// at 4 Hz the gain is gone after an eighth of a second (sample 2756 at 22050 Hz) and back after a quarter
	is!("use sound\ntremolo = 1.0\ns = chord_samples([440], 0.3, \"sine\")\nmax([abs(x - sample_offset) for x in s[2700..2810]]) < 100", true);
	is!("use sound\ntremolo = 1.0\ns = chord_samples([440], 0.3, \"sine\")\nmax([abs(x - sample_offset) for x in s[5400..5600]]) > 5000", true);
}

#[test]
fn vibrato_bends_the_pitch() {
	is!("use sound\nphase(440, 22050)", 440);
	is!("use sound\nvibrato = 1.0\nphase(440, 22050 / 20) > 22.5", true); // ahead of the plain 22 cycles: bent upward first
}

#[test]
fn unison_voices_are_detuned() {
	is!("use sound\nunison = 3\ndetune = 0.2\nv = voices_of([440])\ncount(v) == 3 and v[1] == 440 and v[0] < 440 and v[2] > 440", true);
	is!("unison = 2\nplay(C4, 5ms)\n1", 1);
}

// card out-variable: a program's own `out` and the filter's local `out` stay apart, whatever type the program's holds
#[test]
fn a_program_variable_named_out_survives_the_filter() {
	is!("use sound\nout = [1]\nfiltered([0.1, 0.2])\nout.push(2)\ncount(out)", 2);
	is!("use sound\nout = 7\nchord_samples([440], 0.01, \"sine\")\nout + 1", 8);
	is!("use sound\nout = {}\nout.x = 3\nchord_samples([440], 0.01, \"sine\")\nout.x", 3);
}
