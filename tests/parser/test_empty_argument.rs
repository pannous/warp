//! An empty list or ø after a word is that word's argument, not dropped (card sum-empty)

use crate::is;

#[test]
fn an_empty_argument_of_a_braceless_call() {
	is!("sum []", 0);
	is!("x = sum []; x", 0);
	is!("count []", 0);
	is!("sum [1 2]", 3);
}
