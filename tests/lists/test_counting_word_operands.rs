//! card count-xs: braceless application binds tighter than or, and, comparisons and + -, for any name:
//! `1 + count xs` is `1 + count(xs)`, no "undefined variable count"; a `*` operand stays apart (`h*w int`)
use crate::is;

#[test]
fn a_counting_word_takes_the_name_after_it() {
	is!("xs = [1, 2, 3]; 1 + count xs", 4);
	is!("xs = [1, 2, 3]; 3 == size xs", true);
	is!("xs = [1, 2, 3]; y = 1 + length xs; y", 4);
	is!("w = \"abc\"; 2 - len w", -1);
}

#[test]
fn a_variable_named_count_stays_a_variable() {
	is!("count = 2; 1 + count", 3);
	is!("count = 5; a = 2; [count, a]", warp::ints(vec![5, 2]));
}
