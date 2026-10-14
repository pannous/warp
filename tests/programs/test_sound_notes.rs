// card sound-pro, layer 2 (music values): a note name is parsed, not looked up in a table: any letter A–G with a sharp
// (#, ♯) or flat (b, ♭) and an octave 0–9 is its equal-tempered frequency (A4 = 440 Hz) in a program that uses sound
#![cfg(feature = "native")]
use crate::is;

#[test]
fn sharps_and_flats_are_notes() {
	is!("use sound\nplay(F#4, 5ms)\nround(F#4 * 100)", 36999);
	is!("use sound\nplay(Bb3, 5ms)\nBb3", 233.08);
	is!("use sound\nplay(C♯5, 5ms)\nC♯5 == D♭5", true);
	is!("use sound\nmelody([C4 Eb4 G4], 5ms)\nEb4", 311.13);
}

#[test]
fn every_octave_is_a_note() {
	is!("use sound\nA0", 27.5);
	is!("use sound\nC8", 4186.01);
	is!("use sound\nround(note(\"G#7\").hertz() * 100) == round(G#7 * 100)", true);
}

#[test]
fn a_variable_named_like_a_note_is_the_variable() {
	is!("A4 = 3\nplay(440Hz, 5ms)\nA4", 3);
}

// card play-c4: the body of a def does not define its note names
#[test]
fn a_note_inside_a_function_body_plays() {
	is!("def h(v) { play(C4, 5ms); 5 }; h(2)", 5);
	is!("def h(v) { play(C4, 5ms); v }; h(2)", 2);
	is!("def A4(v) { v }; play(440Hz, 5ms)\nA4(3)", 3);
}
