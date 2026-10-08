// card hint-dismiss: the hint "prefer if x then y else z over x ? y : z" is a got-it note: shown until acknowledged,
// never again after
use warp::diagnostic::{use_acknowledgements_file, with_acknowledger, Acknowledging};
use warp::normalize::capture_hints;

fn ternary_notes(code: &str) -> usize {
	capture_hints(|| warp::wasm_emitter::eval(code)).1.iter().filter(|hint| hint.canonical == "if x then y else z").count()
}

#[test]
fn the_ternary_hint_stays_away_once_acknowledged() {
	let path = "scratch/test_ternary_hint_got_it.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(ternary_notes("x = 1; x > 0 ? 2 : 3"), 1);
		assert_eq!(ternary_notes("x = 1; x > 0 ? 2 : 3"), 1, "not acknowledged yet");
	});
	with_acknowledger(Acknowledging(vec![warp::normalize::CONDITIONAL_TOPIC.to_string()]), || {
		use_acknowledgements_file(path);
		assert_eq!(ternary_notes("x = 1; x > 0 ? 2 : 3"), 1, "shown once more, then got it");
	});
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(ternary_notes("x = 1; x > 0 ? 2 : 3"), 0, "acknowledged in an earlier run");
	});
	let _ = std::fs::remove_file(path);
}
