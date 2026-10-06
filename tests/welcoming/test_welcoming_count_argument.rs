//! `#a` as an argument or list item counts a, like `count a`: `range(1, #a)`, `g(1, #a)`, `[#a, 1]`

use crate::is;

#[test]
fn a_count_inside_brackets_is_a_value_not_a_comment() {
	is!("a=[5,1,2]; g(x,y):=x+y; g(1, #a)", 4);
	is!("a=[5,1,2]; b=[#a, 1]; b[0]", 3);
	is!("a=[5,1,2]; b=[a#1, 1]; b[0]", 5);
	is!("a=[5,1,2]; [#a, 1]", warp::ints(vec![3, 1]));
	is!("# a comment\n3", 3);
}
