// Card clicking-meant: every "I meant" button the page shows can be applied, its text found in the code (a fix whose
// text is not there is a disabled button that only turns its border orange on hover)
use std::collections::HashSet;
use warp::web::evaluate;

const SAMPLE_CHAPTERS: [&str; 10] =
	["errors", "conditions", "numbers", "hello", "lists", "loops", "functions", "text", "types", "matching"];

/// The fixes of a report that the page shows disabled: `start` null
fn disabled_fixes(report: &serde_json::Value) -> Vec<String> {
	let groups = ["errors", "warnings", "hints"];
	groups
		.iter()
		.flat_map(|group| report[group].as_array().cloned().unwrap_or_default())
		.flat_map(|problem| problem["fixes"].as_array().cloned().unwrap_or_default())
		.filter(|fix| fix["start"].is_null())
		.map(|fix| format!("{} (written {})", fix["label"], fix["written"]))
		.collect()
}

#[test]
fn the_samples_fix_buttons_find_their_text() {
	let mut disabled = Vec::new();
	for chapter in SAMPLE_CHAPTERS {
		let mut files: Vec<_> =
			std::fs::read_dir(format!("samples/{chapter}")).unwrap().flatten().map(|entry| entry.path()).collect();
		files.sort();
		for file in files.into_iter().filter(|file| file.extension().is_some_and(|extension| extension == "warp")) {
			let code = std::fs::read_to_string(&file).unwrap();
			for fix in disabled_fixes(&evaluate(&code, HashSet::new())) {
				disabled.push(format!("{}: {fix}", file.display()));
			}
		}
	}
	assert!(disabled.is_empty(), "disabled fix buttons:\n{}", disabled.join("\n"));
}

/// The compiler shows the expression serialized (`3 times 4` for `three times four`, `(pow 2 10)` for `pow(2, 10)`):
/// the fix then edits only the words its reading changes
#[test]
fn a_fix_shown_serialized_edits_the_words_it_changes() {
	use warp::errors::fixits::{fix, fixed};
	let times = fix("multiply", "3 times 4", "3 * 4");
	assert_eq!(fixed("total = three times four", 1, 9, &times).as_deref(), Some("total = three * four"));
	let similar = fix("equal within rounding", "(pow 2 10) == 2^10", "(pow 2 10) ≈ 2^10");
	let code = "# pow(2, 10) == 2**10\ntest pow(2, 10) == 2 ^ 10";
	assert_eq!(fixed(code, 2, 6, &similar).as_deref(), Some("# pow(2, 10) == 2**10\ntest pow(2, 10) ≈ 2 ^ 10"));
}
