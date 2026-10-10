// card hints-dismissed: a plain hint ("prefer xs#1 over xs[0]") is a got-it hint too: "a" silences every hint of its
// reason, "y" the one expression, for good (the acknowledgements file; the CLI keeps it in the home folder)
use warp::diagnostic::{use_acknowledgements_file, with_acknowledger, Acknowledging};
use warp::normalize::capture_hints;

const INDEXING: &str = "xs = [1 2 3]; xs[0] + xs[1]";
const INDEX_TOPIC: &str = "hint:use # for indexing";

fn index_hints(code: &str) -> usize {
	capture_hints(|| warp::wasm_emitter::eval(code)).1.iter().filter(|hint| hint.reason == "use # for indexing").count()
}

#[test]
fn an_acknowledged_kind_of_hint_stays_away() {
	let path = "scratch/test_plain_hint_got_it.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(index_hints(INDEXING), 2, "not acknowledged yet");
	});
	with_acknowledger(Acknowledging(vec![INDEX_TOPIC.to_string()]), || {
		use_acknowledgements_file(path);
		assert_eq!(index_hints(INDEXING), 1, "the first one shown, then got it for all of them");
	});
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(index_hints(INDEXING), 0, "acknowledged in an earlier run");
		assert_eq!(index_hints("ys = [4 5]; ys[1]"), 0, "for every expression of the kind");
	});
	let _ = std::fs::remove_file(path);
}
