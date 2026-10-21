//! `def f(): list? {…}`: a result that can be ø is declared `T?`, also before a block body (decision optional-result)
use crate::is;

#[test]
fn an_optional_result_before_a_block_body_holds_nothing() {
	is!("def f(x: int): list? { found = ø; if x > 0 { found = [x] }; found }; f(0)", warp::Node::Empty);
	is!("def f(x: int): list? { found = ø; if x > 0 { found = [x] }; found }; #f(2)", 1);
	is!("def k(x: int): int? { if x > 1 { return x }; ø }; k(3)", 3);
}

#[test]
fn a_ternary_with_block_branches_stays_a_ternary() {
	is!("c = yes; c ? {1} : {2}", 1);
}
