// card repeating-decimals: a repeating decimal literal is its exact ratio, the period overlined (0.3̅, U+0305 after each
// digit); `as decimal` writes a ratio back that way. No parentheses form (user, 2026-10-11): 0.5(3) multiplies
use crate::is;

#[test]
fn repeating_decimal_literals_are_exact() {
	is!("0.3̅ == 1/3", true);
	is!("1.3̅ == 4/3", true);
	is!("0.16̅ == 1/6", true);
	is!("2.45̅ == 221/90", true);
	is!("-0.3̅ == -1/3", true);
	is!("0.1̅4̅2̅8̅5̅7̅ * 7", 1);
	is!("0.9̅", 1);
	is!("0.3̅ + 0.6̅", 1);
}

#[test]
fn parentheses_after_digits_still_multiply() {
	is!("0.5(3)", 1.5);
}

#[test]
fn ratios_as_decimal_overline_their_period() {
	is!("(1/3) as decimal", "0.3̅");
	is!("(1/6) as decimal", "0.16̅");
	is!("(-22/7) as decimal", "-3.1̅4̅2̅8̅5̅7̅");
	is!("x = 1/3; y = x as decimal; y", "0.3̅");
}

#[test]
fn terminating_and_long_periods_as_decimal() {
	is!("(1/4) as decimal", "0.25");
	is!("3 as decimal", "3");
	is!("(1/7919) as decimal", "1/7919"); // a period of 7918 digits stays a ratio
}

#[test]
fn decimal_text_is_the_prelude_word_the_cast_calls() {
	is!("decimal_text(1, 6)", "0.16̅");
	is!("decimal_text(-3, 8)", "-0.375");
	is!("decimal_text(2, 1019)", "2/1019");
}
