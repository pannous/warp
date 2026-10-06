// `xs.map{it*2}` after a dot is the method call with a block, as `xs.map { it*2 }` (Kotlin, Swift, Ruby); a
// `name{…}` that starts an item stays data (notes/implicit_params.md, card glued-block)
use crate::is;
use warp::ints;

#[test]
fn a_glued_block_after_a_dot_is_a_call() {
	is!("xs=[1,2,3]; xs.map{it*2}", ints(vec![2, 4, 6]));
	is!("[1,2,3].filter{it>1}.map{it*2}", ints(vec![4, 6]));
	is!("xs=[3,1,2]; xs.sort{$0 > $1}", ints(vec![3, 2, 1]));
}

#[test]
fn a_glued_block_starting_an_item_stays_data() {
	is!("x = a{ b:2 c{ d:3 } }; x.c.d", 3);
}
