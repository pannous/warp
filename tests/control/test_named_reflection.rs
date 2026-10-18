//! `help(x)` and `function(name).dump` of a standard module and its words (cards help-undefined, joinable-error,
//! samples/sound/sound_features.warp): a module's help is its index (`warp help sound`), a word's help its head with the
//! comment above it; a name known only at run time (`for trick in dir(sound)`) is looked up in a table of the words
use crate::common::fails_with;
use warp::wasm_emitter::eval;
use warp::Node;

fn text_of(code: &str) -> String {
	match eval(code) {
		Node::Text(text) => text,
		other => panic!("expected a text for {code}, got {other:?}"),
	}
}

fn assert_contains(code: &str, needles: &[&str]) {
	let text = text_of(code);
	for needle in needles {
		assert!(text.contains(needle), "{needle:?} missing in help of {code}: {text}");
	}
}

#[test]
fn help_of_a_standard_module_is_its_index() {
	assert_contains("use sound\nhelp(sound)", &["sound:", "beep()", "tone(pitch, duration, shape)"]);
}

#[test]
fn help_of_a_module_word_has_its_comment() {
	assert_contains("use sound\nhelp(midi)", &["midi(pitch)", "the MIDI note nearest a frequency"]);
}

#[test]
fn help_of_a_name_known_at_run_time() {
	assert_contains("use sound\nword = dir(sound)#18\nhelp(word)", &["midi(pitch)", "the MIDI note nearest"]);
}

#[test]
fn dump_of_a_function_named_at_run_time() {
	assert_contains("use sound\nname = \"beep\"\n(function name).dump", &["beep() := tone(880, 0.15, \"square\")"]);
	assert_contains("use sound\nfunction(\"beep\").dump", &["beep() :="]);
}

#[test]
fn every_word_of_a_module_dumps_and_helps() {
	let code =
		"use sound\nn = 0\nfor trick in dir(sound) { n += count(help(trick)) + count(function(trick).dump) }\nn > 1000";
	assert_eq!(eval(code), Node::True);
}

#[test]
fn a_name_that_is_no_word_is_named_in_the_error() {
	fails_with("use sound\nname = \"bleep\"\nfunction(name).dump", "bleep");
}

#[test]
fn an_unjoinable_item_names_its_kind() {
	fails_with("join([[1], [2]], \",\")", "not a joinable item: a list ([1])");
}

#[cfg(feature = "native")] // the help tests below run the warp binary
const MIDI_DOC: &str = "the MIDI note nearest a frequency";

/// User decision (reflection-plain, replaces help-statement): help(x) gives its text, a statement prints nothing;
/// mid-program `print help(x)` prints
#[cfg(feature = "native")]
#[test]
fn a_help_statement_prints_nothing() {
	let statement = crate::common::printed("use sound\nhelp(midi)\n1");
	assert!(!statement.contains(MIDI_DOC), "{statement}");
	let in_loop = crate::common::printed("use sound\nfor word in [\"midi\"]: print help(word)\n1");
	assert!(in_loop.contains(MIDI_DOC), "{in_loop}");
}

#[cfg(feature = "native")]
#[test]
fn help_in_an_expression_does_not_print() {
	let assigned = crate::common::printed("use sound\nt = help(midi)\n1");
	assert!(!assigned.contains(MIDI_DOC), "{assigned}");
	let printed = crate::common::printed("use sound\nprint help(midi)\n1");
	assert_eq!(printed.matches(MIDI_DOC).count(), 1, "{printed}");
}
