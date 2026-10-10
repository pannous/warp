//! `x and y or z` warns only when y can be falsy (card done-done): `done and "done" or "open"` is a well-defined
//! ternary and says nothing; the warning that stays offers "got it" as an Ask's warning does
use warp::diagnostic::{take_warnings, use_acknowledgements_file, with_acknowledger, Acknowledging};
use warp::wasm_emitter::eval;

const FALSY_MIDDLE: &str = "x = 1; x and 0 or 2";

fn and_or_warnings(code: &str) -> usize {
	take_warnings();
	eval(code);
	take_warnings().iter().filter(|warning| warning.message.contains("is falsy")).count()
}

#[test]
fn a_truthy_literal_in_the_middle_needs_no_warning() {
	assert_eq!(eval("done = 0; done and \"done\" or \"open\"").serialize(), "\"open\"");
	assert_eq!(and_or_warnings("done = 0; done and \"done\" or \"open\""), 0);
	assert_eq!(and_or_warnings(FALSY_MIDDLE), 1);
}

#[test]
fn the_and_or_warning_stays_away_once_acknowledged() {
	let path = "scratch/test_and_or_got_it.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(and_or_warnings(FALSY_MIDDLE), 1, "not acknowledged yet");
	});
	with_acknowledger(Acknowledging(vec![warp::analyzer::AND_OR_TOPIC.to_string()]), || {
		use_acknowledgements_file(path);
		assert_eq!(and_or_warnings(FALSY_MIDDLE), 1, "shown once more, then got it");
	});
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(and_or_warnings(FALSY_MIDDLE), 0, "acknowledged in an earlier run");
	});
	let _ = std::fs::remove_file(path);
}
