// card nested-index: `m#2#1 = 9` (or m[1][0] = 9) sets the item inside the inner list, as `t = m#2; t#1 = 9; m#2 = t`
// does; it used to change a copy and leave m as it was. `row = m#2; row#1 = 9` still leaves m alone (values are never
// shared)
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn a_nested_index_assignment_sets_the_inner_item() {
	is!("m = [[1, 2], [3, 4]]; m#2#1 = 9; m", parse("[[1 2] [9 4]]"));
	is!("m = [[1, 2], [3, 4]]; m[1][0] = 9; m", parse("[[1 2] [9 4]]"));
	is!("m = [[1, 2], [3, 4]]; m#2#1 += 5; m", parse("[[1 2] [8 4]]"));
	is!("c = [[[1, 2]], [[3, 4]]]; c#2#1#2 = 7; c", parse("[[[1 2]] [[3 7]]]"));
	is!("m = [[1, 2], [3, 4]]; row = m#2; row#1 = 9; m", parse("[[1 2] [3 4]]"));
}
