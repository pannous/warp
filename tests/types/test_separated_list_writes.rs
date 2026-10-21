//! A list that keeps its separator (`{1; 2}`, parser.warp's nodes) carries no element mark: in a program declaring a
//! list type its writes stay unchecked (card mark-separator, blocked warp pure)
use crate::is;

#[test]
fn a_list_keeping_its_separator_takes_any_item() {
	is!("names: texts = [\"a\"]; f(l) := { l.add(2); l }; count f(List([1], \"{\" as Bracket, \";\" as Separator))", 2);
	is!("names: texts = [\"a\"]; xs = List([1], \"{\" as Bracket, \";\" as Separator); xs.add(\"b\"); count xs", 2);
}
