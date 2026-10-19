// card note-value (sound layer 2), user decision: the word note. note(pitch) is a Note (key, cents) that prints as its name, plays as its
// frequency, moves by semitones with + and -, and interval(low, high) counts the semitones between two notes. Bare note
// names stay frequencies. A program calling play without `use sound` keeps its note names (the module's class goes in
// front alone, the module stays a used module)
#![cfg(feature = "native")]
use crate::is;

#[test]
fn a_note_value_prints_as_its_name() {
	is!("use sound\nnote(C4)", "C4");
	is!("use sound\nnote(C♯4)", "C#4");
	is!("use sound\nnote(\"Eb5\")", "D#5"); // a note-name text is the name
	is!("use sound; note_value(E4)", "E4"); // the first word, an alias
	is!("use sound\nnote(445Hz)", "A4+20c");
	is!("use sound\nnote(435Hz)", "A4-20c");
	is!("use sound\nn = note(C4)\nn.key", 60);
	is!("use sound\nn = note(C4)\nn.octave()", 4);
	is!("use sound\nn = note(C♯4)\nn.name()", "C#");
}

#[test]
fn semitones_move_a_note() {
	is!("use sound\nnote(C♯4) + 2", "D#4");
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

// card note-value-follow: a note name in a variable, notes as interval's ends, a part of a semitone, no silent 0 Hz
#[test]
fn a_note_name_in_a_variable_is_a_note() {
	is!("use sound; s = \"C#4\"; note(s)", "C#4");
	is!("use sound; s = \"A4\"; hertz_of(s)", 440);
}

#[test]
fn notes_are_the_ends_of_an_interval() {
	is!("use sound; interval(note(C4), note(G4))", 7);
	is!("use sound; interval(C4, note(E4))", 4);
}

#[test]
fn a_part_of_a_semitone_is_cents() {
	is!("use sound; note(C4) + 2.25", "D4+25c");
	is!("use sound; note(C4) - 0.25", "C4-25c");
	is!("use sound; note(C4) + 7", "G4");
}

#[test]
fn something_else_is_no_pitch() {
	let errors =
		crate::common::warp_command().args(["--no-ask", "eval", "use sound; hertz_of(\"hello\")"]).output().unwrap();
	let printed = String::from_utf8_lossy(&errors.stdout).to_string() + &String::from_utf8_lossy(&errors.stderr);
	assert!(printed.contains("not a pitch"), "{printed}");
}
