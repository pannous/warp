//! A hole `$name` in a `data(…)` pattern binds the subtree there, and a repeated hole must be the same subtree
//! (card code-holes, tree-patterns step 7, notes/parse_word.md)
use crate::is;

const SIMPLIFY: &str = "simplify(code) := match code {\n\tdata($a + 0) => a\n\tdata($a * 1) => a\n\t_ => code\n}\n";

#[test]
fn a_hole_binds_the_subtree() {
	is!(&format!("{SIMPLIFY}simplify(data (n * 1)) == data n"), true);
	is!(&format!("{SIMPLIFY}simplify(data (n * 2 + 0)).op"), "*");
}

#[test]
fn a_code_pattern_without_a_match_falls_through() {
	is!(&format!("{SIMPLIFY}simplify(data (n * 2)) == data (n * 2)"), true);
	is!(&format!("{SIMPLIFY}simplify(data n) == data n"), true);
}

#[test]
fn a_repeated_hole_must_be_the_same_subtree() {
	let twice = "twice(code) := match code {\n\tdata($x + $x) => \"double\"\n\t_ => \"other\"\n}\n";
	is!(&format!("{twice}twice(data (n + n))"), "double");
	is!(&format!("{twice}twice(data (n + k))"), "other");
}

#[test]
fn is_tests_a_code_pattern() {
	is!("x = data (n * 1)\nx is data($a * 1)", true);
	is!("x = data (n * 2)\nx is data($a * 1)", false);
}

#[test]
fn a_hole_matches_inside_a_call() {
	is!("name(code) := match code {\n\tdata($f($x)) => f\n\t_ => \"none\"\n}\nname(data (g(y))) == data g", true);
}

#[test]
fn an_indexed_copy_of_a_parameter_takes_code() {
	is!("f(c) := { s = c; s#1 }\nf(data (n * 1)) == data n", true);
	is!("f(c) := { s = c; s#1 }\nf([7, 8])", 7);
}
