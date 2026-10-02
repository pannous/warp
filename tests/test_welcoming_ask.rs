// Ask: an ambiguity is a question to the user; unanswerable (tests, CI, pipes) it falls back to a warning or an error
use warp::diagnostic::{ask, reading, with_asker, with_warning_mode, Ask, Fallback, ScriptedAnswers, WarningMode};
use warp::is;
use warp::wasm_emitter::eval;

fn answers(topic: &str, answer: &str) -> ScriptedAnswers {
	ScriptedAnswers(vec![(topic.to_string(), answer.to_string())])
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
fn an_answer_is_remembered_so_the_user_is_not_asked_twice() {
	with_asker(answers("probe", "second"), || {
		assert_eq!(ask(&ambiguity(Fallback::Error)), Ok(1));
		with_asker(ScriptedAnswers(vec![]), || assert!(ask(&ambiguity(Fallback::Error)).is_err()));
		assert_eq!(ask(&ambiguity(Fallback::Error)), Ok(1));
	});
}

#[test]
fn answers_persist_in_the_project_answers_file() {
	let path = "scratch/test_welcoming_ask.answers";
	std::fs::create_dir_all("scratch").unwrap();
	std::fs::write(path, "probe = b\n").unwrap();
	warp::diagnostic::use_answers_file(path);
	assert_eq!(ask(&ambiguity(Fallback::Error)), Ok(1));
	with_asker(answers("other", "x"), || {
		warp::diagnostic::use_answers_file(path);
		assert_eq!(ask(&ambiguity(Fallback::Error)), Ok(1));
	});
	std::fs::remove_file(path).unwrap();
}

#[test]
fn upto_asks_and_defaults_to_exclusive() {
	is!("x=0; for i in 1 upto 4 {x+=i}; x", 6);
	with_asker(answers("upto", "inclusive"), || is!("x=0; for i in 1 upto 4 {x+=i}; x", 10));
	with_asker(answers("upto", "..<"), || is!("x=0; for i in 1 upto 4 {x+=i}; x", 6));
	assert!(matches!(eval("use strict\nx=0; for i in 1 upto 4 {x+=i}; x"), warp::Node::Error(_)));
}

#[test]
fn a_kotlin_loop_bound_n_minus_one_asks_about_inclusion() {
	is!("n=4; c=0; for i in 0..n-1 {c+=1}; c", 3);
	with_asker(answers("kotlin-range", "inclusive"), || is!("n=4; c=0; for i in 0..n-1 {c+=1}; c", 4));
	with_asker(answers("kotlin-range", "inclusive"), || is!("n=4; c=0; for i in 0..<n-1 {c+=1}; c", 3));
}

#[test]
fn an_unanswered_error_ask_names_every_explicit_form() {
	let Err(error) = ask(&ambiguity(Fallback::Error)) else { panic!("an Error fallback must not guess") };
	let message = format!("{error:?}");
	assert!(message.contains("`a` for first") && message.contains("`b` for second"), "{message}");
}

#[test]
fn an_educating_note_shows_until_acknowledged() {
	use warp::diagnostic::{educate_once, ACKNOWLEDGED};
	use warp::normalize::capture_hints;
	let note = || capture_hints(|| educate_once("let", "let x", "x", "in wasp let is immutable")).1.len();
	with_asker(ScriptedAnswers(vec![]), || {
		assert_eq!(note(), 1);
		assert_eq!(note(), 0, "once per run");
	});
	let path = "scratch/test_welcoming_ask.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_asker(answers("let", ACKNOWLEDGED), || {
		warp::diagnostic::use_answers_file(path);
		assert_eq!(note(), 1);
	});
	assert!(std::fs::read_to_string(path).unwrap().contains("ack:let = acknowledged"));
	with_asker(ScriptedAnswers(vec![]), || {
		warp::diagnostic::use_answers_file(path);
		assert_eq!(note(), 0, "acknowledged in an earlier run");
	});
	std::fs::remove_file(path).unwrap();
}

#[test]
fn every_for_header_bound_minus_one_asks() {
	let forms = "probes/ask/kotlin_range_forms.wasp"; // 7 loops of n-1 forms, nested in parentheses, `do`, a function
	is!(forms, 21);
	with_asker(answers("kotlin-range", "inclusive"), || is!(forms, 28));
}

#[test]
fn a_runtime_error_names_the_range_hint_and_the_guesses_behind_it() {
	let failure = format!("{:?}", eval("samples/life.wasp"));
	assert!(failure.contains("index out of range"), "{failure}");
	assert!(failure.contains("`..` excludes the end; `...` or `to` include it"), "{failure}");
	assert!(failure.contains("assumed at 23:13, 24:15") && failure.contains("loop bound `..size-1`"), "{failure}");
	is!("a=[1,2]; a#3", warp::error("index out of range"));
}
