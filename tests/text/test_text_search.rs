// A text searched for a text: `contains`, `in` (yes or no), `starts_with`, `ends_with`
use crate::is;

#[test]
fn a_text_contains_a_text() {
	is!("s = \"abc\"; s.contains(\"bc\")", true);
	is!("s = \"abc\"; s.contains(\"x\")", false);
	is!("\"b\" in \"abc\"", true);
	is!("\"cd\" in \"abc\"", false);
}

#[test]
fn a_text_starts_and_ends_with_a_text() {
	is!("s = \"abc\"; s.starts_with(\"ab\")", true);
	is!("s = \"abc\"; s.starts_with(\"b\")", false);
	is!("s = \"abc\"; s.ends_with(\"bc\")", true);
	is!("ends_with(\"abc\", \"abcd\")", false);
	is!("[\"apple\" \"berry\" \"avocado\"].filter(w => w.starts_with(\"a\")).join(\",\")", "apple,avocado");
}

// card starts-with-shows-int: a text test shows yes or no, also outside a condition (decision predicate-bool)
#[test]
fn a_text_test_shows_yes_or_no() {
	let shown = |code: &str| warp::wasm_emitter::eval(code).serialize();
	assert_eq!(shown("\"abc\".starts_with(\"a\")"), "yes");
	assert_eq!(shown("\"abc\".starts_with(\"b\")"), "no");
	assert_eq!(shown("\"abc\".ends_with(\"bc\")"), "yes");
	assert_eq!(shown("\"abc\".contains(\"x\")"), "no");
}
