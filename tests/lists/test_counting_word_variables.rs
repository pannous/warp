// A variable named like a counting word (`count`, `size`) heading a list literal is an item, not a call: `[count, a]`
// was `count(a)` (found by the async worker: `go { count += 1; a = 1 }` passed its shared values as [count, a])
use crate::is;

#[test]
fn a_counting_word_variable_in_a_list_is_an_item() {
	is!("count = 5; a = 2; [count, a]", warp::ints(vec![5, 2]));
	is!("size = 5; a = 2; [size, a]", warp::ints(vec![5, 2]));
	is!("xs = [1, 2, 3]; count xs", 3);
}

#[test]
fn shared_values_named_like_counting_words_reach_a_go_block() {
	is!("shared a = 0; shared count = 0; j = go { count += 1; a = 1; 0 }; await j; count + a", 2);
}
