//! P48 (user-decided, open_decisions.md): the wiki's
//! operator declarations on superscript glyphs (wiki/operator.md) override the built-in superscript power, and the short
//! form `suffix ⁰ := 1` (without `operator`) declares the same.
use warp::*;

#[test]
fn test_suffix_operator_on_a_superscript_digit() {
	is!("suffix operator ³ := it*it*it; 3³", 27);
	is!("suffix operator ⁰ := 1; 3⁰", 1);
	is!("suffix operator ³ := it+1; 3³", 4); // the declaration wins over the built-in power
	is!("suffix operator ³ := it*it*it; 1+2³", 9); // binds tighter than any infix
}

#[test]
fn test_prefix_operator_on_a_superscript_sign() {
	is!("prefix operator ⁻ := it*-1; ⁻3", -3);
	is!("prefix operator ⁻ := it*-1; ⁻3+5", 2);
}

#[test]
fn test_short_declaration_without_the_operator_word() {
	is!("suffix ⁰ := 1; 3⁰", 1);
	is!("suffix ³ := it*it*it; 3³", 27);
}

#[test]
fn test_undeclared_superscripts_stay_powers() {
	is!("3³", 27);
	is!("2⁰", 1);
	is!("suffix operator ‼ := it*2; 2³", 8);
}
