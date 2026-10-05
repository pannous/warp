//! `operator ⊕ has precedence above|below [operator] Y` (wiki/Features.md, row 35, P48 user-decided): a declared operator binds
//! just tighter or looser than Y, a built-in or declared operator. Built-in operators are never re-ranked.
use crate::common::fails_with;
use warp::is;

const PLUS: &str = "infix operator ⊕ := a+b; ";

#[test]
fn test_infix_operator_above_multiplication() {
	is!(&format!("{PLUS}2 * 3 ⊕ 4"), 10); // default (like +): 2*3 + 4
	is!(&format!("{PLUS}operator ⊕ has precedence above *; 2 * 3 ⊕ 4"), 14); // 2 * (3⊕4)
	is!(&format!("{PLUS}operator ⊕ has precedence above operator *; 3 ⊕ 4 * 2"), 14); // (3⊕4) * 2
}

#[test]
fn test_infix_operator_below_addition() {
	is!(&format!("{PLUS}2 ⊕ 3 * 4"), 14);
	is!("infix operator ⊗ := a*b; 2 ⊗ 3 + 1", 7); // default (like +), left to right: (2⊗3)+1
	is!("infix operator ⊗ := a*b; operator ⊗ has precedence below +; 2 ⊗ 3 + 1", 8); // 2 ⊗ (3+1)
	is!("infix operator ⊗ := a*b; operator ⊗ has precedence below +; 1 + 2 ⊗ 3", 9); // (1+2) ⊗ 3
}

#[test]
fn test_precedence_relative_to_a_declared_operator() {
	is!("infix operator ⊕ := a+b; infix operator ⊗ := a*b; operator ⊗ has precedence above ⊕; 1 ⊕ 2 ⊗ 3", 7);
	is!("infix operator ⊕ := a+b; infix operator ⊗ := a*b; operator ⊗ has precedence below ⊕; 1 ⊕ 2 ⊗ 3", 9);
}

#[test]
fn test_declaration_order_does_not_matter() {
	is!("operator ⊕ has precedence above *\ninfix operator ⊕ := a+b\n2 * 3 ⊕ 4", 14);
}

#[test]
fn test_built_in_operators_are_not_re_ranked() {
	fails_with("operator + has precedence above *; 1+2*3", "only a declared operator");
	fails_with("infix operator ⊕ := a+b; operator ⊕ has precedence above ⋈; 1 ⊕ 2", "unknown operator ⋈");
}
