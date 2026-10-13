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

#[test]
fn a_block_after_the_operand_is_the_body() {
	is!("t = [1, 2, 3]; i = 0; while i < count t { i++ }; i", 3);
	is!("t = [1, 2, 3]; i = 0; if i < count t { i = 7 }; i", 7);
	is!("xs = [1, 2]; ys = [x - count xs for x in xs]; ys", warp::ints(vec![-1, 0]));
	is!("x = 2; x is a number", true);
}
