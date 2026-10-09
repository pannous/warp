// card comment-after: prose after `//` stays prose though it holds an operator character (`// GET /api/users`,
// samples/server.warp): two words side by side divide nothing; `// n*2` and `// (n - 1)` still read like a divisor
use warp::diagnostic::{use_acknowledgements_file, with_acknowledger, Acknowledging};
use warp::normalize::capture_hints;

fn slash_notes(code: &str) -> usize {
	let path = "scratch/test_slash_comment_prose_operators.acknowledged";
	std::fs::create_dir_all("scratch").unwrap();
	let _ = std::fs::remove_file(path);
	let notes = with_acknowledger(Acknowledging(vec![]), || {
		use_acknowledgements_file(path);
		capture_hints(|| warp::wasm_emitter::eval(code)).1.iter().filter(|hint| hint.canonical == "a//b").count()
	});
	let _ = std::fs::remove_file(path);
	notes
}

#[test]
fn prose_with_operator_characters_gets_no_division_note() {
	assert_eq!(slash_notes("x = 7 // data for anyone: GET /api/users\nx"), 0);
	assert_eq!(slash_notes("x = 7 // well-known (see notes)\nx"), 0);
	assert_eq!(slash_notes("users = 1\nfun f() {\n\tglobal users // the table, see /api/users\n\tusers\n}\nf()"), 0);
}

#[test]
fn an_expression_after_slashes_still_gets_the_note() {
	assert_eq!(slash_notes("n = 3\nx = 7 // n*2\nx"), 1);
	assert_eq!(slash_notes("n = 3\nx = 7 // (n - 1)\nx"), 1);
}
