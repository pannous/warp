//! card linter-should: `warp format` puts one space after a semicolon and none before it, as after a comma
use warp::formatting::{format, Options};

fn formatted(code: &str) -> String {
	format(code, Options::default()).expect("it parses").text
}

#[test]
fn a_semicolon_is_followed_by_one_space() {
	assert_eq!(formatted("a = 1;b = 2\n"), "a = 1; b = 2\n");
	assert_eq!(formatted("a = 1 ;b = 2\n"), "a = 1; b = 2\n");
	assert_eq!(formatted("a = 1;\n"), "a = 1;\n");
}

#[test]
fn a_semicolon_in_a_text_stays() {
	assert_eq!(formatted("x = \"a;b\"\n"), "x = \"a;b\"\n");
}
