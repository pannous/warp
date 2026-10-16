// card print-sq: after a comparison `and` joins conditions, even before a braceless call (`and sq 3 == 8`); between
// two statements it still runs them in sequence (test_statement_phrases)
use crate::is;

const SQ: &str = "sq(x:int) := x*x; ";

#[test]
fn and_after_a_comparison_joins_braceless_calls() {
	is!(&format!("{SQ}sq 2 == 4 and sq 3 == 8"), false);
	is!(&format!("{SQ}sq 2 == 4 and sq 3 == 9"), true);
	is!(&format!("{SQ}x = sq 2 == 4 and sq 3 == 9; x"), true);
}

#[test]
fn a_group_compares_the_braceless_call() {
	is!(&format!("{SQ}(sq 2 == 4)"), true);
	is!(&format!("{SQ}x = (sq 2 == 4 and sq 3 == 8); x"), false);
	is!(&format!("{SQ}x = (sq 2 == 4 and sq 3 == 9); x"), true);
}

#[cfg(feature = "native")]
#[test]
fn print_shows_the_joined_condition() {
	assert_eq!(crate::common::printed(&format!("{SQ}print sq 2 == 4 and sq 3 == 8")).trim(), "no");
}

#[test]
fn a_call_with_parentheses_takes_the_comparison() {
	is!(&format!("{SQ}sq(2 == 4)"), 0);
}
