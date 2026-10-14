//! A parameter joined to a text literal by + is a text, even when the body also counts it (lib/text.warp's pad_left):
//! without a call to say so it was guessed a list, and the unused definition alone failed with `text + list`
use crate::is;

#[test]
fn counted_parameter_joined_to_a_text_is_a_text() {
	is!("pad(t) := count(t) times \" \" + t; 1", 1);
	is!("pad(t) := t + \"!\" + #t times \".\"; 1", 1);
	is!("pad(t) := count(t) times \" \" + t; pad(\"ab\")", "  ab");
}

#[test]
fn std_text_runs_as_a_program() {
	is!("pad_left(t, width) := if count(t) >= width then t else (width - count(t)) times \" \" + t; 1", 1);
}
