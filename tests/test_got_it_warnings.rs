//! "turn all the Ask into a warning with the got it feature" (user, 2026-10-03): an ambiguity never asks and never
//! remembers an answer that changes what the program means. A guessable one takes its default with a warning that is
//! shown until the user says "got it"; a dangerous one stays an error naming the explicit forms
use warp::diagnostic::{take_warnings, use_acknowledgements_file, with_acknowledger, Acknowledging};
use warp::wasm_emitter::eval;
use warp::*;

const UPTO_LOOP: &str = "x=0; for i in 1 upto 4 {x+=i}; x";

fn upto_warnings(code: &str) -> (Node, usize) {
	take_warnings();
	let value = eval(code);
	(value, take_warnings().iter().filter(|warning| warning.message.contains("upto")).count())
}

fn acknowledging(topics: &[&str]) -> Acknowledging {
	Acknowledging(topics.iter().map(|topic| topic.to_string()).collect())
}

#[test]
fn an_ambiguity_takes_its_default_with_a_warning_naming_the_explicit_form() {
	take_warnings();
	is!(UPTO_LOOP, 6);
	let warnings = take_warnings();
	assert!(warnings.iter().any(|warning| warning.message.contains("taking exclusive") && warning.fix.as_deref() == Some("..<")), "{warnings:?}");
}

#[test]
fn got_it_silences_the_warning_but_never_changes_the_program() {
	let path = "scratch/test_got_it_warnings.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(acknowledging(&["upto"]), || {
		use_acknowledgements_file(path);
		assert_eq!(upto_warnings(UPTO_LOOP), (Node::int(6), 1), "shown once more, then acknowledged");
	});
	with_acknowledger(acknowledging(&[]), || {
		use_acknowledgements_file(path);
		assert_eq!(upto_warnings(UPTO_LOOP), (Node::int(6), 0), "acknowledged in an earlier run: silent, same value");
		assert!(matches!(eval(&format!("use strict\n{UPTO_LOOP}")), Node::Error(_)), "strict never guesses");
	});
	std::fs::remove_file(path).unwrap();
}

#[test]
fn a_dangerous_ambiguity_stays_an_error_even_when_acknowledged() {
	with_acknowledger(acknowledging(&["list-times"]), || {
		match eval("[0]*3") {
			Node::Error(message) => assert!(message.to_string().contains("`3 times [0]` for repeat the list"), "{message}"),
			other => panic!("expected the list-times error, got {other:?}"),
		}
	});
}

#[test]
fn the_explicit_forms_say_it_without_a_warning() {
	take_warnings();
	is!("x=0; for i in 1 ..< 4 {x+=i}; x", 6);
	is!("x=0; for i in 1 ... 4 {x+=i}; x", 10);
	assert_eq!(take_warnings(), vec![]);
}
