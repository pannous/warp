// lib/sound.warp (card basic-sound): `play 440Hz for 0.5s`, `play C4`, `play [C4 E4 G4]`, `melody`, `tone`, `beep`.
// The samples go to the host word sound_samples: natively a WAV file in the temporary folder (warp-sound/sound.wav),
// played only when the warp binary may reach the user, so these tests stay silent; in the browser WebAudio
#![cfg(feature = "native")]
use crate::is;

const SAMPLE_RATE: u32 = 22050;
const WAV_HEADER_BYTES: usize = 44;

/// The samples of the WAV that `program` sounds, run in a temporary folder of its own (tests run at the same time)
fn sounded(folder: &str, program: &str) -> Vec<i16> {
	let folder = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(folder);
	std::fs::create_dir_all(&folder).unwrap();
	let run = crate::common::warp_command().env("TMPDIR", &folder).args(["--no-ask", "eval", program]).output().unwrap();
	let errors = String::from_utf8_lossy(&run.stderr);
	assert!(String::from_utf8_lossy(&run.stdout).contains('7'), "{errors}");
	let path = errors.lines().find_map(|line| line.split_once(" s: ").map(|(_, path)| path)).unwrap_or_else(|| panic!("no sound line: {errors}"));
	let wav = std::fs::read(path).expect("sound.wav written");
	assert_eq!((&wav[0..4], &wav[8..16]), (b"RIFF".as_slice(), b"WAVEfmt ".as_slice()));
	assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), SAMPLE_RATE);
	wav[WAV_HEADER_BYTES..].chunks(2).map(|pair| i16::from_le_bytes([pair[0], pair[1]])).collect()
}

#[test]
fn play_writes_the_tone_as_a_wav() {
	let samples = sounded("sound-play", "play 440Hz for 10ms; 7");
	assert_eq!(samples.len(), 220);
	assert_eq!((samples[0], samples[219]), (0, 0), "faded in and out, so the speaker does not click");
	assert!(samples.iter().any(|&sample| sample > 5000) && samples.iter().any(|&sample| sample < -5000));
}

#[test]
fn a_melody_is_its_notes_one_after_another() {
	assert_eq!(sounded("sound-melody", "melody([C4 E4 [C4 G4]], 20ms); 7").len(), 3 * 441);
	assert_eq!(sounded("sound-melody-each", "melody [C4 E4] each 20ms; 7").len(), 2 * 441);
}

#[test]
fn notes_are_frequencies() {
	is!("use sound\nround(note(\"A4\"))", 440);
	is!("use sound\nround(note(\"C#4\") * 100)", 27718);
	is!("use sound\nround(note(\"Eb5\"))", 622);
	is!("use sound\nC4", 261.63);
}

/// Every note name lib/sound.warp defines (C2 … B6) is its frequency as note(name) computes it, to a hundredth of a Hz
#[test]
fn the_note_table_matches_note() {
	// written out: test_std_coverage looks for each library word in the tests' text; an octave per program keeps each small
	const OCTAVES: [&str; 5] = ["C2 D2 E2 F2 G2 A2 B2", "C3 D3 E3 F3 G3 A3 B3", "C4 D4 E4 F4 G4 A4 B4", "C5 D5 E5 F5 G5 A5 B5", "C6 D6 E6 F6 G6 A6 B6"];
	for octave in OCTAVES {
		let differences: Vec<String> = octave.split(' ').map(|name| format!("abs({name} - note(\"{name}\"))")).collect();
		is!(&format!("use sound\nmax([{}]) < 0.01", differences.join(", ")), true);
	}
}

#[test]
fn sound_words_run_without_use() {
	is!("play C4 for 10ms\nbeep\ntone(220Hz, 10ms, \"square\")\ntone 330Hz for 5ms\nplay [C4 E4 G4] for 10ms\n7", 7);
}
