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
	is!(
		"match parse(\"n * 1\") { data(n * 1) => \"one\"; _ => \"other\" }",
		"one"
	);
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
	is!(
		"t = \"n * 1\"; match parse(t) { data(n * 1) => \"one\"; _ => \"other\" }",
		"one"
	);
}

#[test]
fn a_run_time_parse_failure_is_an_error_try_catches() {
	is!("t = \"f(a, a\"; try { parse(t) } else \"bad\"", "bad");
}

#[test]
fn a_parsed_text_ending_in_warp_is_no_file() {
	is!("t = \"a.warp\"; parse(t).op", ".");
}
