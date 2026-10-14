//! A pattern is a value with a name: `pattern Pair = [_, _]`, then `x is Pair`, an arm `Pair => …`, `it is Pair and Short`
//! (card patterns-values, tree-patterns step 4)
use crate::common::fails_with;
use crate::is;

#[test]
fn is_tests_a_named_pattern() {
	is!("pattern Pair = [_, _]\n[1, 2] is Pair", true);
	is!("pattern Pair = [_, _]\n[1, 2, 3] is Pair", false);
	is!("pattern Pair = [_, _]\nf(x) := x is Pair\nf([1, 2])", true);
}

#[test]
fn an_arm_matches_a_named_pattern() {
	is!("pattern Pair = [_, _]\nmatch [1, 2] { Pair => \"pair\"; _ => \"other\" }", "pair");
	is!("pattern Pair = [_, _]\nmatch [1] { Pair => \"pair\"; _ => \"other\" }", "other");
}

#[test]
fn a_named_pattern_binds_its_names() {
	is!("pattern Tagged = [\"tag\", name, ..]\nmatch [\"tag\", \"xy\", 1] { Tagged => name; _ => \"none\" }", "xy");
}

#[test]
fn named_patterns_combine_with_and() {
	is!(
		"pattern Pair = [_, _]\npattern StartsAtOne = [1, ..]\nxs = [[1, 2], [1, 2, 3], [2, 3]]\ncount(xs where it is Pair and StartsAtOne)",
		1
	);
}

#[test]
fn a_literal_list_stays_a_value() {
	is!("Origin = [0, 0]\nOrigin#2", 0);
}

#[test]
fn a_pattern_without_open_parts_binds() {
	is!("pattern Point = [x, y]\nmatch [3, 4] { Point => x + y }", 7);
}

#[test]
fn a_pattern_assigned_without_the_keyword_is_an_error() {
	fails_with("Pair = [_, _]\n[1, 2] is Pair", "pattern Pair = …");
}

#[test]
fn the_word_pattern_stays_a_call_outside_a_definition() {
	// lib/sound.warp's `pattern "c e g"` is a call of the word; only `pattern Name = …` defines a pattern
	is!(
		"to pattern text: count(text)\npattern Pair = [_, _]\npattern(\"abc\") + count([[1, 2], [3]] where it is Pair)",
		4
	);
}
