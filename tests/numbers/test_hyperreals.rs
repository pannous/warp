//! Hyperreal numbers (wiki/hyperreals.md, notes/hyperreals.md): ε is the canonical infinitesimal, ω = 1/ε, exact
//! Laurent polynomials in ε with exact real coefficients. Ordered by the lowest ε power first (ω terms dominate).
use crate::common::fails_with;
use crate::is;
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn test_epsilon_times_omega_is_one() {
	is!("ε*ω", 1);
	is!("ε*ω == 1", true);
	assert_eq!(shown("1/ε"), "ω");
	is!("1/ε == ω", true);
	assert_eq!(shown("1/ω"), "ε");
}

#[test]
fn test_epsilon_is_positive_and_smaller_than_every_real() {
	is!("ε>0", true);
	is!("ε<0.0001", true);
	is!("-ε<0", true);
	is!("ω>1000000", true);
	is!("1+ε>1", true);
	is!("ε*ε<ε", true);
	is!("ω-1000<ω", true);
	is!("π*ε<ε*4", true);
}

#[test]
fn test_powers_expand_exactly() {
	assert_eq!(shown("(1+ε)^2"), "1+2ε+ε²");
	is!("(1+ε)^2 == 1+2ε+ε²", true);
	assert_eq!(shown("ω^2"), "ω²");
	assert_eq!(shown("2ε"), "2ε");
}

#[test]
fn test_standard_part() {
	is!("st(1+ε)", 1);
	is!("st(3-2ε+ε²)", 3);
	is!("st(ε)", 0);
	fails_with("st(ω)", "infinite");
}

#[test]
fn test_no_silent_approximation_of_hyperreals() {
	fails_with("1/(1+ε)", "infinitely many");
	fails_with("ε as float", "ε");
	fails_with("sin(ε)", "ε");
	fails_with("√ε", "no exact form");
	fails_with("f(x):=x*2; f(ε)", "only supported in constant expressions");
}
