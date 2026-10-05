// P65 (user, 2026-10-05): arithmetic a text can't do on a character (negation, %, /, sqrt) is not_a_number, never its code
// point; ord(c) gives the number; comparisons keep code points; + joins and * repeats as for texts
use crate::common::fails_with;
use warp::is;

#[test]
fn a_character_is_no_number_in_arithmetic() {
	fails_with("-'a'", "not a number");
	fails_with("c='a'; -c", "not a number");
	fails_with("'a' % 2", "not a number");
	fails_with("c='a'; c / 2", "type error: text / int"); // a one-letter text held in a variable
	fails_with("sqrt('a')", "'a'"); // "cannot extract a numeric value from 'a'"
	fails_with("x=[1,'a']; -x#2", "not a number");
	fails_with("x=[1,'a']; x#2 % 2", "not a number");
	is!("x=[1,'a']; try -x#2 else 7", 7);
}

#[test]
fn ord_and_comparisons_keep_code_points() {
	is!("ord('a')", 97);
	is!("-ord('a')", -97);
	is!("c='a'; c >= '0'", 1);
	is!("x=[1,'a']; x#2 > 50", 1);
	is!("'a' + 1", "a1");
}
