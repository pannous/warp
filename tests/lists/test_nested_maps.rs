// A map literal as the value of an entry stays a map: `{a:{b:1, c:2}}` gave a square list [b:1 c:2] for a
use crate::is;

#[test]
fn a_map_inside_a_map_keeps_its_braces() {
	is!("m = {a:{b:1, c:2}}; m.a.c", 2);
	is!("m = {a:{b:1, c:2}, d:3}; \"\\(m)\"", "{a:{b:1 c:2} d:3}");
	is!("m = {a:{b:1, c:2}}; x = m.a; x == {b:1, c:2}", true);
}
