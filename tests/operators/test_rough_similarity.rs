// Card g_YHSM, P211: `~` is looser than `≈`, numbers within 1% (`rough_tolerance`) and texts ignoring surrounding
// whitespace and punctuation too. P212: a class's `approximately` (≈) or `similar` (~) serves both operators when it
// defines only one; with neither, both compare field by field
use crate::is;

#[test]
fn rough_numbers_are_within_one_percent() {
	is!("1 ~ 1.005", true);
	is!("1 ≈ 1.005", false);
	is!("1 ~ 1.02", false);
	is!("1 ~~ 1.005", true);
	is!("x = 100; x ~ 100.9", true);
	is!("rough_tolerance = 0.1; 1 ~ 1.05", true);
	is!("tolerance = 0.1; 1 ≈ 1.05", true);
}

#[test]
fn rough_texts_ignore_surrounding_whitespace_and_punctuation() {
	is!("\"Hello!\" ~ \"hello\"", true);
	is!("\"Hello!\" ≈ \"hello\"", false);
	is!("\"  Hí, \" ~ \"hi\"", true);
	is!("\"a\" ~ \"b\"", false);
	is!("\"a b\" ~ \"ab\"", false);
}

#[test]
fn rough_lists_compare_their_items_roughly() {
	is!("[1 \"Hi!\"] ~ [1.005 \"hi\"]", true);
	is!("[1 \"Hi!\"] ≈ [1.005 \"hi\"]", false);
	is!("{x:1 y:\"Ok.\"} ~ {y:\"ok\" x:1.001}", true);
}

#[test]
fn a_class_method_serves_both_operators_when_only_one_is_defined() {
	let approximately = "class P{x:float; approximately(o) := abs(x - o.x) < 1}; ";
	is!(&format!("{approximately}P(1) ≈ P(1.5)"), true);
	is!(&format!("{approximately}P(1) ~ P(1.5)"), true);
	is!(&format!("{approximately}P(1) ≈ P(3)"), false);
	let similar = "class P{x:float; similar(o) := abs(x - o.x) < 1}; ";
	is!(&format!("{similar}P(1) ~ P(1.5)"), true);
	is!(&format!("{similar}P(1) ≈ P(1.5)"), true);
}

#[test]
fn a_class_with_both_methods_keeps_them_apart() {
	let both = "class P{x:float; approximately(o) := abs(x - o.x) < 1; similar(o) := abs(x - o.x) < 10}; ";
	is!(&format!("{both}P(1) ≈ P(5)"), false);
	is!(&format!("{both}P(1) ~ P(5)"), true);
}

#[test]
fn a_class_without_the_methods_compares_field_by_field_roughly() {
	is!("class P{x:float}; P(1) ~ P(1.005)", true);
	is!("class P{x:float}; P(1) ≈ P(1.005)", false);
}
