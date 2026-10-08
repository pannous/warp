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

/// a prose comment (`// property with value list`, samples/html.warp) reads like no divisor: no note; `// 2`,
/// `// n` and `// n + 1` still do
#[test]
fn a_prose_comment_after_a_value_gets_no_division_note() {
	let path = "scratch/test_slash_comment_prose.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		assert_eq!(slash_notes("x = [1 2]      // property with value list\nx"), 0);
		assert_eq!(slash_notes("x = 7 // seven days\nx"), 0);
		assert_eq!(slash_notes("n = 3\nx = 7 // n\nx"), 1);
		assert_eq!(slash_notes("n = 3\nx = 7 // n + 1\nx"), 1);
	});
	let _ = std::fs::remove_file(path);
}
