//! Methods that change their object: in a branch, from `init(…) := {…}`, an element of a list field (card inference-untyped)
use crate::is;

#[test]
fn a_change_in_a_branch_keeps() {
	is!("class C { items = []\n put(x) := { if not (x in items) { items.add(x) } }\n}\nc = C(); c.put(1); c.put(2); c.put(1); count(c.items)", 2);
}

#[test]
fn init_defined_with_colon_equals_is_the_constructor() {
	is!("class P { x=0; init(v) := { x = v * 2 } }; P(3).x", 6);
	is!("class C { items = []\n put(x) := { if not (x in items) { items.add(x) } }\n init(xs) := { for x in xs { put(x) } }\n}\ncount(C([1, 2, 1]).items)", 2);
}

#[test]
fn an_element_of_a_list_field_changes() {
	is!("class K { counts = [0, 0]\n bump(i) := { counts#i += 1 }\n}\nk = K(); k.bump(1); k.bump(1); k.bump(2); k.counts#1 * 10 + k.counts#2", 21);
	is!("k = {counts: [0, 0]}; k.counts#2 = 7; k.counts#2", 7);
}

#[test]
fn a_counter_of_texts_from_init() {
	let counter = "class Counter { names = []; counts = []\n add(x) := { i = x in names; if i { counts#i += 1 } else { names.add(x); counts.add(1) } }\n init(xs) := { for x in xs { add(x) } }\n}\n";
	is!(&format!("{counter}c = Counter([\"ab\", \"cd\", \"ab\"]); c.counts#1"), 2);
	is!(&format!("{counter}c = Counter([\"a\", \"b\", \"a\"]); c.names#2"), "b");
}
