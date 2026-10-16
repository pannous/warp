// card hint-comment-floordiv: `expr  // result`, a comment set apart by two spaces or a tab, is the samples' habit of
// writing the result beside the code; it gets no floor-division note and stays a comment
use warp::errors::diagnostic::{use_acknowledgements_file, with_acknowledger, Acknowledging};
use warp::normalize::capture_hints;

fn slash_notes_and_value(code: &str) -> (usize, warp::Node) {
	let (value, hints) = capture_hints(|| warp::wasm_emitter::eval(code));
	let notes = hints.iter().filter(|hint| hint.canonical == "a /! b").count();
	(notes, value)
}

#[test]
fn an_aligned_comment_gets_no_division_note() {
	let path = "scratch/test_aligned_comment.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(slash_notes_and_value("x = 5  // 14\nx"), (0, warp::Node::int(5)));
		assert_eq!(slash_notes_and_value("x = 5\t// 14\nx").0, 0);
		assert_eq!(
			slash_notes_and_value("add(a, b) := a + b\ninc = add(1, _)  // partial: inc(x) = add(1, x)\ninc(2)"),
			(0, warp::Node::int(3))
		);
		assert_eq!(slash_notes_and_value("x = 7 // 2\nx").0, 1);
	});
	let _ = std::fs::remove_file(path);
}
