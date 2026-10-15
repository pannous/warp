// One rule for every braceless prefix word (card braceless-prefix, wiki/Footguns.md "Braceless call grabbing too
// little"): the word takes the whole arithmetic argument after it, like `f 3-1` is f(2) and `sin 0 + 1` is sin(1).
// The symbols √ ∛ ‖ stay tight math notation, glued parentheses are the whole argument.
use crate::is;

#[test]
fn operator_words_take_the_whole_argument() {
	is!("sqrt 4 + 5", 3);
	is!("abs -3 + 1", 2);
	is!("cbrt 4 + 4", 2);
	is!("sqrt 2 * 8", 4);
}

#[test]
fn operator_words_stop_where_calls_stop() {
	is!("sqrt 16 > 3", true);
	is!("1 + sqrt 4 + 5", 4);
}

#[test]
fn glued_parentheses_are_the_whole_argument() {
	is!("sqrt(16) + 1", 5);
	is!("abs(-3) + 1", 4);
}

#[test]
fn symbols_stay_tight() {
	is!("√4 + 5", 7);
	is!("‖-3‖ + 1", 4);
}
