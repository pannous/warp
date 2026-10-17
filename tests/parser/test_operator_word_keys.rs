// card abs-fails: a prefix operator word (abs sqrt cbrt norm not, if, while) directly before a key colon names the key,
// as an infix operator word does (`{from:1 to:2}`); `abs x` stays the operator
use crate::is;

#[test]
fn a_prefix_operator_word_names_a_key() {
	is!("{abs:5}.abs", 5);
	is!("x = {a:1, abs:5}; x.abs", 5);
	is!("{sqrt: 9}.sqrt", 9);
	is!("{not:1}.not", 1);
	is!("{if:2}.if", 2);
}

#[test]
fn the_prefix_operator_word_stays_an_operator() {
	is!("abs -5", 5);
	is!("x = {v: -2}; abs x.v", 2);
}
