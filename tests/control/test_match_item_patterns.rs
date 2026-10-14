//! An item pattern `[[n, s]...]` matches when every item of the list matches it; a name it binds holds the list of
//! every item's value (card list-pattern, tree-patterns step 3)
use crate::is;

#[test]
fn every_item_binds_a_list() {
	is!("names = match [[1, \"a\"], [2, \"b\"]] { [[n, s]...] => s }\nnames#2", "b");
	is!("numbers = match [[1, \"a\"], [2, \"b\"]] { [[n, s]...] => n }\ncount(numbers)", 2);
}

#[test]
fn one_failing_item_misses_the_arm() {
	is!("match [[1, 2], [3]] { [[a, b]...] => 1; _ => 0 }", 0);
	is!("match [[1, 2], 3] { [[a, b]...] => 1; _ => 0 }", 0);
}

#[test]
fn items_after_fixed_parts() {
	is!("match [0, [1, 2], [3, 4]] { [head, [a, b]...] => head + b#2 }", 4);
}

#[test]
fn literal_items_test_every_item() {
	is!("match [[1, \"x\"], [1, \"y\"]] { [[1, s]...] => count(s); _ => 0 }", 2);
	is!("match [[1, \"x\"], [2, \"y\"]] { [[1, s]...] => count(s); _ => 0 }", 0);
}

#[test]
fn item_pattern_in_is() {
	is!("pairs = [[1, 2], [3, 4]]\nif pairs is [[a, b]...] then a#2 else 0", 3);
}

#[test]
fn map_items_collect_their_fields() {
	is!(
		"order = {id: 7, items: [{sku: \"a1\", qty: 2}, {sku: \"b2\", qty: 1}]}\nskus = match order { {items: [{sku}...]} => sku }\nskus#2",
		"b2"
	);
	is!(
		"order = {items: [{sku: \"a1\"}, {qty: 1}]}\nmatch order { {items: [{sku}...]} => \"all\"; _ => \"missing\" }",
		"missing"
	);
}
