//! D1 (user decisions 2026-10-03): double-quoted text interpolates Swift `"\(expr)"` and `"${expr}"`; a bare `"$x"`
//! is text ("only the one with the curly braces must interpolate the other is text like dollar money").
//! `\(…)` is the canonical form the normalizer hints. Single quotes stay literal; `$` holes in sql/sh templates and
//! `$0` arguments keep working; `$` before a digit or a space stays a dollar sign.

use crate::common::fails_with;
use crate::is;
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasp_parser::{parse_data, WaspParser};
use warp::Node;

fn hints_of(code: &str) -> Vec<(String, String)> {
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| WaspParser::parse(code));
	hints.into_iter().map(|hint| (hint.original, hint.canonical)).collect()
}

#[test]
fn swift_holes_interpolate_any_expression() {
	is!("x=3; \"a \\(x+1) b\"", "a 4 b");
	is!("name=\"Bob\"; \"hi \\(name)!\"", "hi Bob!");
	is!("\"\\(1+2)\"", "3");
	is!("x=2; \"\\(x)\\(x*2)\"", "24");
	is!("\"sum: \\((1+2)*3)\"", "sum: 9");
}

#[test]
fn curly_dollar_holes_interpolate_bare_dollars_stay_text() {
	is!("x=3; \"a ${x+1} b\"", "a 4 b");
	is!("name=\"Bob\"; \"hi $name!\"", "hi $name!"); // user decision: "the other is text like dollar money"
	is!("n=7; \"$n items\"", "$n items");
	is!("x=3; \"$x and ${x}\"", "$x and 3");
}

#[test]
fn holes_take_parameters_lists_and_exact_numbers() {
	is!("f(n):=\"n is \\(n)\"; f(4)", "n is 4");
	is!("xs=[1 2]; \"xs=\\(xs)\"", "xs=[1 2]");
	is!("y=2.5; \"v=\\(y)\"", "v=2.5");
	is!("\"a \\(\"b\") c\"", "a b c"); // a text inside a hole
}

#[test]
fn holes_take_text_results_of_library_words() {
	is!("name=\"bob\"; \"hi \\(name.upper)\"", "hi BOB");
}

#[test]
fn single_quotes_stay_literal() {
	is!("'hi $name'", "hi $name");
	is!("'a ${x} b'", "a ${x} b");
}

#[test]
fn a_dollar_before_a_digit_or_space_is_a_dollar_sign() {
	is!("\"costs $5\"", "costs $5");
	is!("\"$ 5\"", "$ 5");
	is!("\"a\\$b\"", "a$b"); // an escaped dollar is never a hole
}

#[test]
fn an_unknown_name_in_a_hole_is_loud() {
	fails_with("\"hi ${nobody}\"", "nobody");
}

#[test]
fn data_keeps_dollars_as_text() {
	assert_eq!(parse_data("\"$ref\"").drop_meta().clone(), Node::Text("$ref".into()));
}

#[test]
fn dollar_arguments_and_templates_keep_working() {
	is!("add1(x):=$0+1; add1(3)", 4);
	let injected = "name=\"x' OR '1'='1\";q=sql \"SELECT * FROM t WHERE name = $name\"";
	is!(&format!("{injected};q#1"), "SELECT * FROM t WHERE name = ?");
	is!(&format!("{injected};q#2"), "x' OR '1'='1");
	is!("name=\"x\";q=sql \"SELECT * FROM t WHERE name = \\(name)\";q#1", "SELECT * FROM t WHERE name = ?");
	is!("file=\"a b\";c=sh \"rm -f \\(file)\";c#3", "a b");
	// a bare $name stays a sql/sh hole, also next to curly ones
	is!("a=\"x\";b=\"y\";q=sql \"SELECT * FROM t WHERE a = $a AND b = ${b}\";q#3", "y");
	is!("a=\"x\";b=\"y\";q=sql \"SELECT * FROM t WHERE a = $a AND b = ${b}\";q#1", "SELECT * FROM t WHERE a = ? AND b = ?");
}

#[test]
fn the_normalizer_hints_the_swift_form() {
	assert_eq!(hints_of("x=1; \"a ${x} b\""), vec![("${x}".to_string(), "\\(x)".to_string())]);
	assert_eq!(hints_of("x=1; \"a $x b\""), vec![]); // a bare $x is text, nothing to hint
	assert_eq!(hints_of("x=1; \"a \\(x) b\""), vec![]);
}
