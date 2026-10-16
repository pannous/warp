//! Decision operator-alias (card standalone-math-units): `operator × := *` spells an operator another way, keeping its
//! precedence, `prefix operator √ := sqrt` a prefix one; lib/math.warp's alias lines are the math spellings every
//! program reads, in place of the parser's Rust table
use crate::is;
use warp::operators::{math_aliases, spelled_operator, Op};

#[test]
fn the_math_spellings_come_from_lib_math() {
	assert!(math_aliases().iter().any(|alias| alias.spelling == "×" && alias.op == Op::Mul));
	assert_eq!(spelled_operator("≤"), Some(Op::Le));
	assert_eq!(spelled_operator("*"), Some(Op::Mul));
}

#[test]
fn a_math_spelling_keeps_its_operator_and_precedence() {
	is!("1 + 2 × 3", 7);
	is!("6 ÷ 3 − 1", 1);
	is!("2 ≤ 3 ∧ 4 ≠ 5", true);
	is!("√16 + ∛27", 7.0);
}

#[test]
fn a_program_declares_its_own_alias() {
	is!("operator ⊕ := +\n1 ⊕ 2 * 3", 7);
	is!("prefix operator ∂ := sqrt\n∂16", 4.0);
}

#[test]
fn lib_math_with_its_alias_lines_still_loads() {
	is!("use math; 2 × square 3", 18);
	is!("include math; 2 × square 3", 18);
}
