// card note-value (sound layer 2), user decision: the word note. note(pitch) is a Note (key, cents) that prints as its name, plays as its
// frequency, moves by semitones with + and -, and interval(low, high) counts the semitones between two notes. Bare note
// names stay frequencies. A program calling play without `use sound` keeps its note names (the module's class goes in
// front alone, the module stays a used module)
#![cfg(feature = "native")]
use crate::is;

#[test]
fn a_note_value_prints_as_its_name() {
	is!("use sound\nnote(C4)", "C4");
	is!("use sound\nnote(C#4)", "C#4");
	is!("use sound\nnote(\"Eb5\")", "D#5"); // a note-name text is the name
	is!("use sound\nnote_value(E4)", "E4"); // the first word, an alias
	is!("use sound\nnote(445Hz)", "A4+20c");
	is!("use sound\nnote(435Hz)", "A4-20c");
	is!("use sound\nn = note(C4)\nn.key", 60);
	is!("use sound\nn = note(C4)\nn.octave()", 4);
	is!("use sound\nn = note(C#4)\nn.name()", "C#");
}

#[test]
fn semitones_move_a_note() {
	is!("use sound\nnote(C#4) + 2", "D#4");
	is!("use sound\nn = note(C4)\nm = n + 7\nm", "G4");
	is!("use sound\nnote(E4) - 4 == note(C4)", true);
	is!("use sound\ninterval(C4, G4)", 7);
}

#[test]
fn a_note_value_plays_as_its_frequency() {
	is!("use sound\nnote(A4).hertz()", 440);
	is!("use sound\nplay(note(A4), 5ms)\nplay([note(C4), note(E4)], 5ms)\n7", 7);
}

#[test]
fn note_names_stay_with_play_alone() {
	is!("play(C4, 5ms)\nplay([C4 E4], 5ms)\n7", 7);
}

#[test]
fn a_class_printing_method_keeps_is_text() {
	// card is-text-method: Note's text() made `x is text` an equality, so play "song.wav" played a tone
	is!("class N { k: int\n text() := \"n\" }\nf(x) := if x is text then 1 else 2\nf(\"a\")", 1);
}
