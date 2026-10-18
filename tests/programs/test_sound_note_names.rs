// Every note name of lib/sound.warp is its MIDI note's frequency (decision music-plain-details): the letter, a flat
// `b` or a sharp `s` (`♯` and `♭` spell the same notes) and the octave 0–8
use crate::is;

const NOTE_NAMES: [&str; 9] = [
	"C0 Cs0 Db0 D0 Ds0 Eb0 E0 F0 Fs0 Gb0 G0 Ab0 Gs0 A0 As0 Bb0 B0",
	"C1 Cs1 Db1 D1 Ds1 Eb1 E1 F1 Fs1 Gb1 G1 Ab1 Gs1 A1 As1 Bb1 B1",
	"C2 Cs2 Db2 D2 Ds2 Eb2 E2 F2 Fs2 Gb2 G2 Ab2 Gs2 A2 As2 Bb2 B2",
	"C3 Cs3 Db3 D3 Ds3 Eb3 E3 F3 Fs3 Gb3 G3 Ab3 Gs3 A3 As3 Bb3 B3",
	"C4 Cs4 Db4 D4 Ds4 Eb4 E4 F4 Fs4 Gb4 G4 Ab4 Gs4 A4 As4 Bb4 B4",
	"C5 Cs5 Db5 D5 Ds5 Eb5 E5 F5 Fs5 Gb5 G5 Ab5 Gs5 A5 As5 Bb5 B5",
	"C6 Cs6 Db6 D6 Ds6 Eb6 E6 F6 Fs6 Gb6 G6 Ab6 Gs6 A6 As6 Bb6 B6",
	"C7 Cs7 Db7 D7 Ds7 Eb7 E7 F7 Fs7 Gb7 G7 Ab7 Gs7 A7 As7 Bb7 B7",
	"C8 Cs8 Db8 D8 Ds8 Eb8 E8 F8 Fs8 Gb8 G8 Ab8 Gs8 A8 As8 Bb8 B8",
];

/// The MIDI note of a note name: C4 is 60, a flat one below, a sharp one above
fn midi_of(name: &str) -> i64 {
	let letter = "C D EF G A B".find(&name[..1]).unwrap() as i64;
	let accidental = match &name[1..2] {
		"b" => -1,
		"s" => 1,
		_ => 0,
	};
	let octave: i64 = name[name.len() - 1..].parse().unwrap();
	12 * (octave + 1) + letter + accidental
}

#[test]
fn every_note_name_is_its_midi_notes_frequency() {
	for names in NOTE_NAMES {
		let midis: Vec<String> = names.split(' ').map(|name| midi_of(name).to_string()).collect();
		let program =
			format!("use sound\n[{}] == [midi_hertz(n) for n in [{}]]", names.replace(' ', ", "), midis.join(", "));
		is!(&program, true);
	}
}

#[test]
fn a_letter_indexed_by_an_octave_is_its_sharp_note() {
	is!("use sound\nF#4 == F♯4", true);
	is!("use sound\n[C#0, D#3, G#8, A#4] == [Cs0, Ds3, Gs8, As4]", true);
	is!("use sound\nC#4 + 0", 277.18);
}
