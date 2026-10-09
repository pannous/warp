// card index-range (user, 2026-10-07): "index out of range" names the index and the range of the list it missed
use crate::common::fails_with;

#[test]
fn an_index_out_of_range_names_the_range() {
	fails_with("xs = [1, 2, 3]; i = 7; xs#i", "index out of range: 7 not in 1…3");
	fails_with("xs = [\"a\", \"b\"]; xs#5", "index out of range: 5 not in 1…2");
	fails_with("xs = [1, 2, 3]; xs[3]", "index out of range: 4 not in 1…3"); // [] counts from 0, the message from 1
	fails_with("xs = []; i = 1; xs#i", "index out of range: 1 not in an empty list");
}
