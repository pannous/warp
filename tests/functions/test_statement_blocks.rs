// Card statement-block: a block of one statement runs it as a program of that statement does, also when its word is a
// phrase (`to add a to b`) or has several definitions (`to play x`, `to play x for duration`): `f() := {play "x.wav"}`
// plays, it returned the data `{play "x.wav"}`; `x = {…}` still assigns the data as written
use crate::is;
use warp::wasm_emitter::eval;

const BOOP: &str = "to boop x: x + 1; to boop x for y: x + y; ";
const MISSING_FILE_ERROR: &str = "cannot open x.wav";

#[test]
fn a_block_of_one_phrase_call_runs_it() {
	is!("to add a to b: a + b; f() := {add 1 to 2}; f()", 3);
	is!(&format!("{BOOP}f() := {{boop 2}}; f()"), 3);
	is!(&format!("{BOOP}f() := {{boop 2 for 5}}; f()"), 7);
	is!(&format!("{BOOP}if 1 {{boop 2}}"), 3);
}

#[test]
fn a_block_of_one_sound_word_plays() {
	for program in ["f() := {play \"x.wav\"}; f()", "if 1 {play \"x.wav\"}", "job = go { play \"x.wav\" }; await job"] {
		assert!(eval(program).serialize().contains(MISSING_FILE_ERROR), "{program}");
	}
}

#[test]
fn an_assigned_block_stays_data() {
	is!(&format!("{BOOP}x = {{boop 2}}; count(x)"), 2);
}
