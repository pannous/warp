// card music-values (sound layer 2): notes as music values. The tuning (A4's frequency, 440 unless set) tunes every
// note name; midi(pitch) and note_name(pitch) read a frequency as its note; transpose moves notes by semitones;
// chord(root, kind) and scale(root, kind) are lists of notes, their kind written as a word or a text
#![cfg(feature = "native")]
use crate::is;

#[test]
fn the_tuning_tunes_every_note() {
	is!("tuning = 442\nplay(A4, 5ms)\nA4", 442);
	is!("tuning = 432\nplay(440Hz, 5ms)\nA3", 216);
	is!("use sound\nmidi_hertz(60)", 261.63); // the word a note name lowers to
}

#[test]
fn a_frequency_reads_as_its_note() {
	is!("use sound\nmidi(C4)", 60);
	is!("use sound\nmidi(440Hz)", 69);
	is!("use sound\nnote_name(Eb4)", "D#4");
	is!("use sound\nnote_name(27.5)", "A0");
}

#[test]
fn transposing_moves_notes_by_semitones() {
	is!("use sound\ntranspose(A4, 12)", 880);
	is!("use sound\ntranspose(C4, 12) == C5", true);
	is!("use sound\ntranspose([A4, A3], -12) == [A3, A2]", true);
	is!("use sound\nround(transpose(300Hz, 12))", 600);
}

#[test]
fn chords_and_scales_are_lists_of_notes() {
	is!("use sound\nchord(C4, major7) == [C4, E4, G4, B4]", true);
	is!("use sound\nchord(A3, \"minor\") == [A3, C4, E4]", true);
	is!("use sound\nscale(D4, dorian) == [D4, E4, F4, G4, A4, B4, C5, D5]", true);
	is!("use sound\nplay(chord(C4, minor), 5ms)\nmelody(scale(C4, major), 5ms)\ncount(scale(A2, pentatonic))", 6);
}
