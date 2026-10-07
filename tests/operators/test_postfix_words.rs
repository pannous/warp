//! card postfix-words: an operator word after its value (`-7 abs`) and a trailing percent (`10 %`)
use crate::is;

#[test]
fn an_operator_word_after_its_value_applies_to_it() {
	is!("-7 abs", 7);
	is!("x = -7 abs; x", 7);
	is!("16 sqrt", 4.0);
	is!("x = -3; x abs + 1", 4);
	is!("27 cbrt", 3.0);
	is!("abs -7", 7);
}

#[test]
fn a_trailing_percent_is_a_hundredth() {
	is!("10 %", 0.1);
	is!("10%", 0.1);
	is!("200 * 10%", 20);
	is!("x = 50%; x * 4", 2);
	is!("10% + 1", 1.1);
	is!("7 % 5", 2);
	is!("x = 7; x %= 5; x", 2);
}
