//! card lib-math: a prelude word answers to its own name, not only to the library spelling that leads to it
//! (`round_to(x, 2)` as `round(x, 2)`), so lib/math.warp needs no copy of round_to
use crate::is;

#[test]
fn prelude_words_answer_to_their_own_names() {
	is!("round_to(1.23456, 2)", 1.23);
	is!("use math; round_to(1.23456, 2)", 1.23);
	is!("first_items([5, 6, 7], 2)", warp::ints(vec![5, 6]));
	is!("last_items([5, 6, 7], 2)", warp::ints(vec![6, 7]));
}

#[test]
fn a_program_word_named_like_a_prelude_word_stays_the_programs() {
	is!("round_to(x, d) := 7; round_to(1.5, 2)", 7);
}
