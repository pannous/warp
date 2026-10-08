use warp::warp_parser::parse;

#[test]
fn semicolon_at_line_end_separates_statements_like_the_newline() {
	assert_eq!(parse("a;\nb\nc").size(), 3);
	assert_eq!(parse("package p;\ninterface i {a}\nworld w {b}").size(), 3);
}

#[test]
fn semicolon_inside_a_line_still_groups() {
	assert_eq!(parse("a;b\nc").size(), 2);
}
