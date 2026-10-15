//! Card operator-infix (user: "operator == infix operator by default"): `operator ⊕ := …` without a kind declares an
//! infix operator, as `infix operator ⊕ := …` does
use crate::is;

#[test]
fn an_operator_without_a_kind_is_infix() {
	is!("operator ⊕ := a*10+b; 2 ⊕ 3", 23);
	is!("operator ⊕(x: int, y: int) := x * 10 + y; 4 ⊕ 2", 42);
	is!("operator divides(d:int, n:int) := n % d == 0; 3 divides 12", true);
}

#[test]
fn a_precedence_declaration_stays_one() {
	is!("operator ⊕ := a+b; operator ⊕ has precedence above *; 2 ⊕ 3 * 2", 10);
}
