// Ask and educate state belongs to one compilation: two evals on one thread (a test thread reused by the harness, a
// REPL) must not see each other's assumptions, remembered answers or shown notes, and the hint configuration of one
// thread is not another's (found on Linux CI 2026-10-03: one program's "assumed at …" in another test's error).
use std::cell::Cell;
use warp::diagnostic::{with_asker, Ask, Asker};
use warp::normalize::{capture_hints, hint_mode, set_hint_mode, HintMode};
use warp::wasm_emitter::eval;
use warp::Node;

const KOTLIN_BOUND: &str = "n=3; s=0; for i in 0..n-1 { s+=i }; s";

fn let_notes(code: &str) -> usize {
	capture_hints(|| eval(code)).1.iter().filter(|hint| hint.original.starts_with("let ")).count()
}

#[test]
fn an_eval_does_not_see_the_assumptions_of_the_one_before() {
	eval(KOTLIN_BOUND);
	match eval("xs=[1 2]; xs#5") {
		Node::Error(message) => assert!(!message.to_string().contains("assumed at"), "{message}"),
		other => panic!("expected the index error, got {other:?}"),
	}
}

#[test]
fn every_eval_shows_its_educate_once_note() {
	assert_eq!(let_notes("let x = 3; x"), 1);
	assert_eq!(let_notes("let y = 4; y"), 1, "the note of the first program hid the second one's");
}

/// Answers the first question it is asked, then nothing
struct AnswersOnce(Cell<bool>);

impl Asker for AnswersOnce {
	fn answer(&self, ask: &Ask) -> Option<usize> {
		(!self.0.replace(true)).then_some(ask.readings.len() - 1)
	}
}

#[test]
fn an_answer_given_in_one_eval_is_not_remembered_by_the_next() {
	with_asker(AnswersOnce(Cell::new(false)), || {
		assert_eq!(eval(KOTLIN_BOUND), 3, "answered inclusive: 0+1+2");
		assert_eq!(eval(KOTLIN_BOUND), 1, "unanswered: the default, exclusive: 0+1");
	});
}

#[test]
fn the_hint_mode_of_one_thread_is_not_another_threads() {
	std::thread::spawn(|| set_hint_mode(HintMode::Off)).join().unwrap();
	assert_eq!(hint_mode(), HintMode::Always);
}
