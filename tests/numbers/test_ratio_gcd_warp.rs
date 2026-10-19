//! The exact runtime's int_gcd, ratio_new, exact_div, int_mod and int_euclid_div written in warp (lib/math.warp, card
//! ratio-gcd-warp) agree with the builtins they twin, for fixnums and for ints beyond 64 bits
use crate::common::fails_with;
use crate::is;

/// The parts of the RatioParts r as the text "n/d"
const PARTS: &str = "\"\\(r.numerator)/\\(r.denominator)\"";

#[test]
fn gcd_is_never_negative() {
	is!("use math; gcd(-12, 18)", 6);
	is!("use math; gcd(12, -18)", 6);
	is!("use math; gcd(-4, 0)", 4);
	is!("use math; gcd(0, 0)", 0);
	is!("use math; gcd(2^70, 3 * 2^65) == 2^65", true);
}

#[test]
fn lowest_terms_are_the_ratios_of_division() {
	is!(&format!("use math; r = lowest_terms(6, -4); {PARTS}"), "-3/2");
	is!("use math; r = lowest_terms(6, -4); r.numerator / r.denominator == 6 / -4", true);
	is!(&format!("use math; r = lowest_terms(2^70, 3 * 2^65); {PARTS}"), "32/3");
	is!(&format!("use math; r = lowest_terms(-5, 0); {PARTS}"), "-1/0");
	is!(&format!("use math; r = lowest_terms(0, 0); {PARTS}"), "0/0");
}

#[test]
fn exact_quotient_divides_as_slash() {
	is!(&format!("use math; r = exact_quotient(RatioParts(1, 2), RatioParts(-3, 4)); {PARTS}"), "-2/3");
	is!(&format!("use math; r = exact_quotient(RatioParts(6, 1), RatioParts(3, 1)); {PARTS}"), "2/1");
	fails_with("use math; exact_quotient(RatioParts(1, 2), RatioParts(0, 1))", "divide by zero");
}

#[test]
fn euclidean_words_match_percent_and_floor_division() {
	let all_signs = "use math; same = true
for a in [-9, -7, 0, 7, 9] { for b in [-3, -2, 2, 3] {
	if euclidean_remainder(a, b) != a % b or euclidean_quotient(a, b) != a /! b { same = false }
} }
same";
	is!(all_signs, true);
	is!("use math; n = -(2^70); euclidean_remainder(n, 1000)", 576);
	is!("use math; n = -(2^70); euclidean_quotient(n, 3) == n /! 3", true);
	is!("use math; euclidean_quotient(-7, 2)", -4);
}
