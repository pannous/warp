// card sound-pro, layer 2 (music values): a note name is parsed, not looked up in a table: any letter A–G with a sharp
// (#, ♯) or flat (b, ♭) and an octave 0–9 is its equal-tempered frequency (A4 = 440 Hz) in a program that uses sound
#![cfg(feature = "native")]
use crate::is;

#[test]
fn sharps_and_flats_are_notes() {
	is!("play F#4 for 5ms\nround(F#4 * 100)", 36999);
	is!("play Bb3 for 5ms\nBb3", 233.08);
	is!("play C♯5 for 5ms\nC♯5 == D♭5", true);
	is!("melody [C4 Eb4 G4] each 5ms\nEb4", 311.13);
}

#[test]
fn every_octave_is_a_note() {
	is!("use sound\nA0", 27.5);
	is!("use sound\nC8", 4186.01);
	is!("use sound\nround(note(\"G#7\") * 100) == round(G#7 * 100)", true);
}

#[test]
fn a_variable_named_like_a_note_is_the_variable() {
	is!("A4 = 3\nplay 440Hz for 5ms\nA4", 3);
}
