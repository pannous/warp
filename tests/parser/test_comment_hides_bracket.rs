// A spaced ` // ` inside brackets starts a comment; when that comment hides the closing bracket the writer meant floor
// division: a loud parse error naming the `//`, not a validation failure or a misleading undefined function
use crate::common::fails_with;
use warp::is;

#[test]
fn a_comment_that_hides_a_closing_bracket_is_an_error() {
	fails_with("col=7; (col // 3) * 3", "starts a comment that hides the closing `)`");
	fails_with("a=[1 2 3]; n=4; a[n // 2]", "hides the closing `]`");
}

#[test]
fn a_comment_after_an_open_bracket_without_its_closer_is_a_comment() {
	is!("x = [1, 2, // first two\n3]; x#3", 3);
	is!("col=7; (col//3)*3", 6);
}
