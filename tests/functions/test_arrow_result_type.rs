//! card undefined-function: the result arrow before `:=`, `def x(p:P) -> int := p.n`, is the typed head
//! `x(p:P): int := p.n`; it was read as a lambda and the call said `undefined function: x`
use crate::is;

#[test]
fn a_result_arrow_before_define() {
	is!("class P{n:int}; def x(p:P) -> int := p.n; x(P(2))", 2);
	is!("class P{n:int}; x(p:P) -> int := p.n; x(P(2))", 2);
	is!("def x(p:int) -> int := p+1; x(2)", 3);
	is!("def x(p) -> int := p+1; x(2)", 3);
}

#[test]
fn arrow_lambdas_stay() {
	is!("f = (x) -> x+1; f(2)", 3);
	is!("f := x -> x*2; f(3)", 6);
	is!("def x(p:int) -> int { p+1 }; x(2)", 3);
}
