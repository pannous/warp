// card prepositions-call (user 2026-10-14) and play-tone: a parameter written after a preposition, `f(x, for y)`, is
// passed by that word, also in a call without brackets; elsewhere the words keep their meaning (P52: `to` a range).
// src/lowering/definitions/preposition_calls.rs
use crate::common::fails_with;
use crate::is;

#[test]
fn a_preposition_names_its_argument() {
	is!("f(x, for y) := x - y\nf 5 for 2", 3);
	is!("f(x, for y) := x - y\nf(5, for: 2)", 3);
	is!("f(x, for y) := x - y\nf(5, for=2)", 3);
	is!("f(x, for y) := x - y\nf(for: 2, x: 5)", 3);
	is!("f(x, for y) := x - y\nf(5, y: 2)", 3);
	is!("f(x, for y) := x - y\nf(5, 2)", 3);
	is!("f(x, by y) := x * y\nf 3 by 4", 12);
	is!("f(x, each y) := x * y\nf 3 each 4", 12);
	is!("f(x, with y) := x * y\nf 3 with 4", 12);
}

#[test]
fn several_prepositions_in_any_order() {
	is!("move(x, from a, to b) := x + a - b\nmove 1 from 10 to 3", 8);
	is!("move(x, from a, to b) := x + a - b\nmove 1 to 3 from 10", 8);
	is!("move(x, from a, to b) := x + a - b\nr = move 1 from 10 to 3\nr", 8);
	is!("f(x, to y) := x - y\nr = f 9 to 4\nr", 5);
	is!("f(x, in y) := x * y\nr = f 3 in 4\nr", 12);
}

#[test]
fn a_phrase_definition_takes_its_prepositions() {
	is!("to move piece from a to b: piece + a - b\nmove 1 from 10 to 3", 8);
	is!("to add a to b: a + b\nadd 1 to 2", 3);
	is!("to add a to b: a + b\nadd(1, 2)", 3);
	is!("to hum notes each d: notes * d\nhum 3 each 4", 12);
}

#[test]
fn a_call_without_brackets_names_arguments() {
	is!("f(a, b) := a - b\nf b=1 a=5", 4);
	is!("f(a, b) := a - b\nf b:1 a:5", 4);
}

#[test]
fn a_call_with_prepositions_inside_another_call() {
	is!("f(x, to y) := x - y\ng(a) := a * 10\ng(f 9 to 4)", 50);
	is!("f(x, to y) := x - y\ng(a) := a * 10\ng((f 9 to 4))", 50);
}

#[test]
fn the_words_keep_their_meaning_outside_such_calls() {
	is!("f(x, to y) := x - y\nxs = 1 to 4\ns = 0\nfor x in xs { s += x }\ns + (f 9 to 4)", 15);
	is!("f(x, to y) := x - y\nsum(1 to 4) + f(9, 4)", 15);
	is!("f(x, in y) := x * y\nif 3 in [1, 3] then f 3 in 4 else 0", 12);
	is!("f(x, each y) := x * y\ns = 0\n[1, 2].each(x => s += x)\ns + f 2 each 3", 9);
}

#[test]
fn a_preposition_the_function_does_not_take_is_an_error() {
	fails_with("f(x, with y) := x - y\nf 5 by 2", "f takes no argument after by; it takes one after with");
	fails_with("f(x, for y) := x - y\nf 5 for", "for needs a value after it");
	fails_with("for = 3", "for is a keyword");
}

/// play needs no `use sound`, and a label is no mention of the sound word tone (reviewer)
#[test]
fn play_names_its_arguments_without_use_sound() {
	is!("play tone:440Hz for:0.1s\n1", 1);
	is!("play tone=440Hz for=0.1s\n1", 1);
	is!("play(tone: C4, for: 0.1s)\n1", 1);
	fails_with("tone(440Hz, 0.1s)", "tone is in the standard module sound");
}

#[test]
fn a_variable_named_like_a_preposition_stays_a_variable() {
	is!("by = 2\nf(x, with y) := x - y\nf 5 by", 3);
}

#[cfg(feature = "native")] // renders into a temporary file, each in a run of its own
#[test]
fn play_and_melody_take_their_durations_after_for_and_each() {
	let seconds = |name: &str, sounding: &str| {
		crate::programs::test_sound_voices::rendered(name, &format!("use sound\n{sounding}\nrender_sound(\"SONG\")")).0
	};
	assert_eq!(seconds("for", "play 440Hz for 0.3s"), "0.3");
	assert_eq!(seconds("colons", "play tone:440Hz for:0.3s"), "0.3");
	assert_eq!(seconds("equals", "play tone=440Hz for=0.3s"), "0.3");
	assert_eq!(seconds("brackets", "play(440Hz, for: 0.3s)"), "0.3");
	assert_eq!(seconds("chord", "play [C4 E4 G4] for 0.3s"), "0.3");
	assert_eq!(seconds("melody", "melody [G4 E4 C4] each 100ms"), "0.3");
	assert_eq!(seconds("tone", "play(tone: C4)"), "0.5");
	assert_eq!(seconds("signal", "hum(t) := sin(t * 440 * tau)\nplay hum for 0.3s"), "0.3");
}
