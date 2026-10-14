// found merging decimals-exact (lib/sound.warp filtered: `x2 = x1` before `x1 = x` in a loop): a local widened to a Node
// by a later write in the loop is a Node for the reads before that write too, as a widening to a float already was
use crate::is;

#[test]
fn a_read_before_a_node_widening_in_a_loop_holds_the_node() {
	is!("def g(xs) {\nx1 = 0\nx2 = 0\nfor x in xs { x2 = x1\nx1 = x }\nx2 }\ng([1.0f, 2.5f]) == 1.0f", true);
	is!("def g(xs) {\nx1 = 0.0\nx2 = 0.0\nfor x in xs { x2 = x1\nx1 = x }\nx2 }\ng([1, 2.5f])", 1);
}
