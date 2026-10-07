// The values of a pair are Nodes of any type: `y = values(a: {b: 8})#1` holds the pair b: 8, not an int (it was the
// runtime error "not an int"; lib/markup.wasp reads an element's block this way)
use crate::is;

#[test]
fn a_value_of_a_pair_is_held_as_a_node() {
	is!("p = (a: { b: 8 }); y = values(p)#1; keys(y)#1", "b");
	is!("p = (a: { padding: \"q\" }); x = values(p); y = x#1; values(y)#1", "q");
	is!("v = [\".x\": { padding: 8 }]; n = 0; for item in v { inner = values(item)#1; if inner is pair { n = n + 1 } }; n", 1);
	is!("m = {a: 1, b: 2}; v = values(m)#2; v + 1", 3);
}
