// A text searched for a text: `contains`, `in` (the 1-based position), `starts_with`, `ends_with`
use warp::is;

#[test]
fn a_text_contains_a_text() {
	is!("s = \"abc\"; s.contains(\"bc\")", 1);
	is!("s = \"abc\"; s.contains(\"x\")", 0);
	is!("\"b\" in \"abc\"", 2);
	is!("\"cd\" in \"abc\"", 0);
}

#[test]
fn a_text_starts_and_ends_with_a_text() {
	is!("s = \"abc\"; s.starts_with(\"ab\")", 1);
	is!("s = \"abc\"; s.starts_with(\"b\")", 0);
	is!("s = \"abc\"; s.ends_with(\"bc\")", 1);
	is!("ends_with(\"abc\", \"abcd\")", 0);
	is!("[\"apple\" \"berry\" \"avocado\"].filter(w => w.starts_with(\"a\")).join(\",\")", "apple,avocado");
}
