//! Card data-op: a parenthesized expression in quoted code, `(a * 2)` in `data [(a * 2)]`, stays the group as written,
//! and the Node API reads through a group of one item: its op, key, value and name are the item's

use crate::is;

#[test]
fn a_group_of_one_has_its_item_s_parts() {
	is!("x = data [(a * 2)]; (x#1).op", "*");
	is!("x = data [(a * 2)]; (x#1).value", 2);
	is!("x = data [((a * 2))]; (x#1).op", "*");
	is!("t = \"[(a * 2)]\" + \"\"\nr = parse(t)\n(r#1).op", "*");
	is!("x = data [(a * 2)]; (x#1).bracket", "(");
}
