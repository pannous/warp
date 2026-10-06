// card samples-html: the note "`// …` after code is a comment" is for `a // b`, which reads like a division; after a
// text, an opening brace or a comma nothing could be divided, so the comment gets no note
use warp::diagnostic::{use_acknowledgements_file, with_acknowledger, Acknowledging};
use warp::normalize::capture_hints;

fn slash_notes(code: &str) -> usize {
	capture_hints(|| warp::wasm_emitter::eval(code)).1.iter().filter(|hint| hint.canonical == "a//b").count()
}

#[test]
fn a_comment_after_a_text_gets_no_division_note() {
	let path = "scratch/test_slash_comment_after_value.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(slash_notes("div{ class:\"form-group\" // free form values need to be quoted\n}"), 0);
		assert_eq!(slash_notes("xs = [1, // the first\n2]\nxs"), 0);
		assert_eq!(slash_notes("x = 7 // 2\nx"), 1);
	});
	let _ = std::fs::remove_file(path);
}
