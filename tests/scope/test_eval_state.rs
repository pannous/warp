// Ask and educate state belongs to one compilation: two evals on one thread (a test thread reused by the harness, a
// REPL) must not see each other's assumptions or shown notes, and the hint configuration of one
// thread is not another's (found on Linux CI 2026-10-03: one program's "assumed at …" in another test's error).
use warp::diagnostic::{take_warnings, with_acknowledger, Acknowledging};
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

#[test]
fn a_got_it_without_a_file_lasts_one_eval_and_never_changes_the_value() {
	let bound_warnings = || take_warnings().iter().filter(|warning| warning.message.contains("loop bound")).count();
	with_acknowledger(Acknowledging(vec!["kotlin-range".to_string()]), || {
		take_warnings();
		assert_eq!(eval(KOTLIN_BOUND), 1, "the default, exclusive: 0+1");
		assert_eq!(bound_warnings(), 1);
		assert_eq!(eval(KOTLIN_BOUND), 1, "the same value every time");
		assert_eq!(bound_warnings(), 1, "nothing remembered it");
	});
}

#[cfg_attr(not(feature = "native"), ignore = "browser: spawns a thread")]
#[test]
fn the_hint_mode_of_one_thread_is_not_another_threads() {
	std::thread::spawn(|| set_hint_mode(HintMode::Off)).join().unwrap();
	assert_eq!(hint_mode(), HintMode::Always);
}
