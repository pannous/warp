// Ask: an ambiguity takes its default with a "got it" warning or is an error naming the explicit forms; it never asks
// and never remembers a reading (user 2026-10-03: no context-sensitive execution)
use warp::diagnostic::{ask, reading, take_warnings, use_acknowledgements_file, with_acknowledger, with_warning_mode, Acknowledging, Ask, Fallback, WarningMode};
use crate::is;
use warp::wasm_emitter::eval;

fn acknowledging(topic: &str) -> Acknowledging {
	Acknowledging(vec![topic.to_string()])
}

fn ambiguity(fallback: Fallback) -> Ask {
	Ask::new("probe", "which reading?", vec![reading("first", "a"), reading("second", "b")], fallback)
}

#[test]
fn an_unanswered_ask_falls_back_to_its_default_or_its_error() {
	assert_eq!(ask(&ambiguity(Fallback::Warning)), Ok(0));
	assert!(ask(&ambiguity(Fallback::Error)).is_err());
	assert!(with_warning_mode(WarningMode::Error, || ask(&ambiguity(Fallback::Warning))).is_err());
}

#[test]
fn an_acknowledged_ambiguity_still_compiles_the_same_way() {
	with_acknowledger(acknowledging("probe"), || {
		assert_eq!(ask(&ambiguity(Fallback::Warning)), Ok(0));
		assert_eq!(ask(&ambiguity(Fallback::Warning)), Ok(0));
		assert!(ask(&ambiguity(Fallback::Error)).is_err());
	});
}

#[test]
fn an_old_answers_line_never_picks_a_reading() {
	let path = "scratch/test_welcoming_ask.answers";
	std::fs::create_dir_all("scratch").unwrap();
	std::fs::write(path, "probe = b\n").unwrap();
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert!(ask(&ambiguity(Fallback::Error)).is_err());
		assert_eq!(ask(&ambiguity(Fallback::Warning)), Ok(0));
	});
	std::fs::remove_file(path).unwrap();
}

#[test]
fn upto_asks_and_defaults_to_exclusive() {
	is!("x=0; for i in 1 upto 4 {x+=i}; x", 6);
	is!("x=0; for i in 1 ... 4 {x+=i}; x", 10); // the explicit inclusive form
	is!("x=0; for i in 1 ..< 4 {x+=i}; x", 6);
	assert!(matches!(eval("use strict\nx=0; for i in 1 upto 4 {x+=i}; x"), warp::Node::Error(_)));
}

#[test]
fn a_kotlin_loop_bound_n_minus_one_asks_about_inclusion() {
	is!("n=4; c=0; for i in 0..n-1 {c+=1}; c", 3);
	is!("n=4; c=0; for i in 0...n-1 {c+=1}; c", 4); // the explicit inclusive form
	is!("n=4; c=0; for i in 0..<n-1 {c+=1}; c", 3);
}

#[test]
fn an_unanswered_error_ask_names_every_explicit_form() {
	let Err(error) = ask(&ambiguity(Fallback::Error)) else { panic!("an Error fallback must not guess") };
	let message = format!("{error:?}");
	assert!(message.contains("`a` for first") && message.contains("`b` for second"), "{message}");
}

#[test]
fn an_educating_note_shows_until_acknowledged() {
	use warp::diagnostic::educate_once;
	use warp::normalize::capture_hints;
	let note = || capture_hints(|| educate_once("let", "let x", "x", "in warp let is immutable")).1.len();
	with_acknowledger(Acknowledging(vec![]), || {
		assert_eq!(note(), 1);
		assert_eq!(note(), 0, "once per run");
	});
	let path = "scratch/test_welcoming_ask.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(acknowledging("let"), || {
		use_acknowledgements_file(path);
		assert_eq!(note(), 1);
	});
	assert!(std::fs::read_to_string(path).unwrap().contains("ack:let = acknowledged"));
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(note(), 0, "acknowledged in an earlier run");
	});
	std::fs::remove_file(path).unwrap();
}

#[test]
fn every_for_header_bound_minus_one_asks() {
	let forms = "probes/ask/kotlin_range_forms.warp"; // 7 loops of n-1 forms, nested in parentheses, `do`, a function
	take_warnings();
	is!(forms, 21);
	let warned = take_warnings().iter().filter(|warning| warning.message.contains("include")).count();
	assert_eq!(warned, 7, "every loop warns");
}

#[test]
fn a_runtime_error_names_the_range_hint_and_the_guesses_behind_it() {
	let failure = format!("{:?}", eval("samples/life_kotlin_ranges.warp"));
	assert!(failure.contains("index out of range"), "{failure}");
	assert!(failure.contains("`..` excludes the end; `...` or `to` include it"), "{failure}");
	assert!(failure.contains("assumed at 23:13, 24:15") && failure.contains("loop bound `..size-1`"), "{failure}");
	is!("a=[1,2]; a#3", warp::error("index out of range: 3 not in 1…2"));
}

#[test]
fn questions_quote_prefix_operators_as_written() {
	let value_text = |code: &str| match warp::warp_parser::WarpParser::parse(code).drop_meta() {
		warp::Node::Key(_, _, value) => warp::normalize::operand_text(value),
		other => panic!("not an assignment: {other:?}"),
	};
	assert_eq!(value_text("k=#a-1"), "#a-1");
	assert_eq!(value_text("k=n-1"), "n-1");
}
