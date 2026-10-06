// User decisions 2026-10-03 (notes/open_decisions.md): #25/#36 a space before `[` never indexes,
// #27 Unicode operators normalized to ASCII and `be` as `:=`, D8 similarity with a relative tolerance.
use warp::wasp_parser::parse;
use crate::{is, eq};

#[test]
fn space_before_bracket_passes_the_list() {
	is!("first [10, 5]", 10);
	is!("last [10, 5]", 5);
	is!("first [10 5]", 10);
	is!("reduce [7] (a b)->a+b", 7);
	is!("reduce [7, 3] (a b)->a+b", 10);
	is!("xs=[10 5]; xs[1]", 5);
}

#[test]
fn unicode_operators_are_their_ascii_spelling() {
	for (unicode, ascii) in [("3 ≤ 4", "3 <= 4"), ("3 ≥ 4", "3 >= 4"), ("3 ≠ 4", "3 != 4"), ("3 × 4", "3 * 4"), ("8 ÷ 4", "8 / 4"), ("¬ x", "not x"), ("√ x", "sqrt x")] {
		eq!(parse(unicode), parse(ascii));
	}
	is!("3 ≤ 4", true);
	is!("4 ≥ 5", false);
	is!("3 ≠ 4", true);
	is!("3 × 4", 12);
	is!("8 ÷ 4", 2);
	is!("¬ false", true);
	is!("√16", 4);
}

#[test]
fn be_defines_like_colon_equals() {
	eq!(parse("x be 3"), parse("x := 3"));
	is!("x be 3; x", 3);
	is!("twice be it*2; twice 4", 8);
}

#[test]
fn similarity_has_relative_tolerance() {
	is!("1 ≈ 1.0000000001", true);
	is!("1 ≈ 1.001", false);
	is!("1000000000 ≈ 1000000001", true);
	is!("1000 ≈ 1001", false);
	is!("0.1+0.2 ≈ 0.3", true);
	is!("1 ~ 1.0000000001", true);
	is!("1 ~ 1.1", false);
	is!("1 circa 1.0000000001", true);
	is!("1 approximately 1.0000000001", true);
	is!("2 ≈ 2", true);
}

#[test]
fn similarity_tolerance_is_settable() {
	is!("tolerance = 0.01; 100 ≈ 100.5", true);
	is!("tolerance = 0.01; 100 ≈ 102", false);
	is!("tolerance = 0.1; 1 circa 1.05", true);
}

#[test]
fn similarity_combines_and_takes_any_operand() {
	is!("1 ≈ 2 or 3 ≈ 3", true);
	is!("π ≈ 3.14159265358979", true);
	is!("π ≈ 3.14", false);
	is!("x=0; x ≈ 0", true);
	is!("x=0; x ≈ 1e-12", false);
	is!("xs=[1 2]; xs#2 ≈ 2.0000000001", true);
	is!("1 ~~ 1.0000000001", true);
	is!("1 ⋍ 1.0000000001", true);
}
