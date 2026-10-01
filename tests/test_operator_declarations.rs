//! `prefix|suffix|infix operator ⊕ := body` declares an operator (wiki operator.md); the parser learns it from a pre-scan
use warp::*;
mod common;
use common::fails_with;

#[test]
fn test_suffix_operator() {
	is!("suffix operator ‼ := it*2; 3‼", 6);
	is!("suffix operator ‼ := it*2; x=4; x‼+1", 9);
	is!("suffix operator ‼ := it*2; (1+2)‼", 6);
}

#[test]
fn test_prefix_operator() {
	is!("prefix operator ∆ := it*it; ∆3", 9);
	is!("prefix operator ∆ := it*it; ∆3+1", 10);
}

#[test]
fn test_infix_operator() {
	is!("infix operator ⊕ := a+b; 2 ⊕ 3", 5);
	is!("infix operator ⊕ := a*10+b; 2 ⊕ 3", 23);
	is!("infix operator ⊕ := a+b; 2 ⊕ 3*2", 8);
	is!("infix operator ⊕ := a+b; x=1; x ⊕ 1 ⊕ 1", 3);
}

#[test]
fn test_infix_operator_declared_by_its_pattern() {
	is!("x ⊗ y := x*y; 2 ⊗ 4", 8);
}

#[test]
fn test_operators_declared_later_are_known_to_the_whole_program() {
	is!("suffix operator ‼ := it*2\n3‼", 6);
}

#[test]
fn test_precedence_declarations_are_refused() {
	fails_with("infix operator ⊕ := a+b; operator ⊕ has precedence above +", "operator precedence declarations are not supported yet");
}

#[test]
fn test_built_in_operators_are_untouched() {
	is!("3²", 9);
	is!("2+3*4", 14);
	is!("infix operator ⊕ := a+b; 3²", 9);
}
