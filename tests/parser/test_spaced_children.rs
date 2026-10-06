// inside a data literal a spaced child `c { d:3 }` is the child node of the glued `c{ d:3 }` (card spaced-child,
// wiki/reference.md writes them spaced); in code `run { … }` stays a call with a block
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn a_spaced_child_in_a_data_literal_is_a_child_node() {
	is!("x = a{ b:2 c { d:3 } }; x.c.d", 3);
	is!("x = a{ b:2 c{ d:3 } }; x.c.d", 3);
	is!("x = a{ c { d { e:5 } } }; x.c.d.e", 5);
	assert_eq!(parse("a{ b:2 c { d:3 } }").serialize(), parse("a{ b:2 c{ d:3 } }").serialize());
}
