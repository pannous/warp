//! Card parse-word (notes/parse_word.md): `parse(text)` gives warp code the Node of a source, the same Node
//! `data (…)` gives, so code can be read, compared and matched as data
use crate::is;

#[test]
fn parse_of_a_constant_text_is_its_code_as_data() {
	is!("parse(\"n * 1\") == data (n * 1)", true);
	is!("x = parse \"a + 0\"; x.op", "+");
	is!("x = parse(\"f(a, a)\"); x#1", warp::parse("f"));
	is!("parse(\"n * 1\")", warp::parse("n * 1"));
	is!("parse(\"n * 1\") == data (n * 2)", false);
	is!("parse(\"n * 1\").value", 1);
}

#[test]
fn quoted_code_in_parentheses_is_a_node() {
	is!("(data (n * 1)).op", "*");
	is!("(data (n * 1)) == data (n * 1)", true);
}

#[test]
fn a_hole_is_a_symbol_of_the_parse() {
	is!("parse(\"$a + 0\").key.name", "$a");
}

#[test]
fn parse_matches_like_data() {
	is!("match parse(\"n * 1\") { data(n * 1) => \"one\"; _ => \"other\" }", "one");
}

#[test]
fn a_program_s_own_parse_wins() {
	is!("parse(t) := 7; parse(\"x\")", 7);
}

// card parse-runtime: a text known only at run time is parsed by the host word parse_code
#[test]
fn parse_of_a_run_time_text_is_its_code_as_data() {
	is!("t = \"n * \" + \"1\"; parse(t) == data (n * 1)", true);
	is!("t = \"$a + 0\"; parse(t).key.name", "$a");
	is!("t = \"n * 1\"; match parse(t) { data(n * 1) => \"one\"; _ => \"other\" }", "one");
}

#[test]
fn a_run_time_parse_failure_is_an_error_try_catches() {
	is!("t = \"f(a, a\"; try { parse(t) } else \"bad\"", "bad");
}

#[test]
fn a_parsed_text_ending_in_warp_is_no_file() {
	is!("t = \"a.warp\"; parse(t).op", ".");
}

// card data-meta: quoted code keeps where it was written and the comment before it
#[test]
fn quoted_code_knows_its_line_and_column() {
	is!("x = data (n * 1); x.column", 11);
	is!("y = 2\nx = data (n * 1); x.line", 2);
	is!("x = data (n * 1); x.value.column", 15);
	is!("x = parse(\"a; b + 1\"); y = x#2; y.column", 4);
	is!("x = data (n * 1); x == data (n * 1)", true);
}

#[test]
fn quoted_code_keeps_the_comment_before_it() {
	is!("use comments\nx = data {\n  // the a\n  a: 1\n}\nx#1.comment", "the a");
}

// card name-quoted: a parenthesized element is read like the bare one
#[test]
fn a_parenthesized_element_of_quoted_code_has_its_parts() {
	is!("x = data (a; b); (x#2).name", "b");
	is!("x = {a: 1, b: 2}; (x#2).key", warp::parse("b"));
	is!("x = [{a: 7}]; (x#1).a", 7);
}

#[test]
fn a_constant_text_that_fails_to_parse_fails_where_try_catches() {
	is!("try { parse(\"f(a, a\") } else \"bad\"", "bad");
}
