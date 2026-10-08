//! The standard library module regex (notes/stdlib.md section 7): Rust's regex natively, JS RegExp in the browser,
//! held to what both engines do alike: look-around and backreferences are a loud error in both
use crate::is;
use warp::warp_parser::parse;

#[test]
fn use_regex_matches_finds_and_replaces() {
	is!("use regex; matches(\"order 66\", \"[0-9]+\")", true);
	is!("use regex; matches(\"no digits\", \"[0-9]+\")", false);
	is!("use regex; first_match(\"order 66, then 7\", \"[0-9]+\")", "66");
	is!("use regex; first_match(\"none\", \"[0-9]+\")", parse("ø"));
	is!("use regex; find_all(\"a11 b22 c333\", \"[0-9]+\")", parse("[\"11\" \"22\" \"333\"]"));
	is!("use regex; replace_all(\"a1 b22\", \"[0-9]+\", \"#\")", "a# b#");
	is!("use regex; replace_all(\"John Smith\", \"(\\\\w+) (\\\\w+)\", \"$2 $1\")", "Smith John");
}

#[test]
fn what_one_engine_lacks_is_an_error_in_both() {
	crate::common::fails_with("use regex; matches(\"ab\", \"a(?=b)\")", "look-around");
	crate::common::fails_with("use regex; matches(\"aa\", \"(a)\\\\1\")", "backreference");
}
