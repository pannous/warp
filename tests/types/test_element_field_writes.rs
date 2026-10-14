//! A field of a list element changes like an element of an element (card list-field-receivers): `bags#1.n = 5` takes
//! the instance out, changes it and puts it back; the store into a call's result `make().n = 5` changes nothing, so it
//! is an error. The element checks of a declared list field hold for these receivers too.
use crate::common::fails_with;
use crate::is;

const BAGS: &str = "class bag { n: int; items: texts }; bags = [bag(1, [\"a\"])]; ";

#[test]
fn a_field_of_a_list_element_changes() {
	is!(&format!("{BAGS}bags#1.n = 5; bags#1.n"), 5);
	is!(&format!("{BAGS}bags#1.n += 2; bags#1.n"), 3);
	is!(&format!("{BAGS}bags#1.items.add(\"b\"); first = bags#1; #first.items"), 2);
	is!(&format!("{BAGS}bags#1.items#1 = \"z\"; first = bags#1; first.items#1"), "z");
	is!(&format!("{BAGS}i = 1; bags#i.n = 7; bags#1.n"), 7);
}

#[test]
fn a_list_field_of_an_element_checks_its_items() {
	fails_with(&format!("{BAGS}bags#1.items.add(420); bags"), "items of bag");
	fails_with(&format!("same(y) := y; {BAGS}bags#1.items.add(same(420)); bags"), "items of bag");
	is!(&format!("same(y) := y; {BAGS}bags#1.items.add(same(\"b\")); first = bags#1; #first.items"), 2);
}

#[test]
fn a_character_stored_into_a_list_of_unknown_type_stays_a_character() {
	is!("class bag { items: list }; b = bag([\"a\"]); x = b.items; x#1 = \"z\"; x#1", "z");
	is!("class bag { items: texts }; b = bag([\"a\"]); b.items#1 = \"z\"; b.items#1", "z");
	is!("class p { s: text }; q = p(\"ab\"); y = q.s; y#1 = \"z\"; y", "zb");
}

#[test]
fn a_store_into_a_call_result_is_an_error() {
	fails_with("class bag { n: int }; make() := bag(1); make().n = 5", "make() gives a copy");
	fails_with("class bag { items: texts }; make() := bag([\"a\"]); make().items.add(\"b\")", "make() gives a copy");
	is!("class bag { n: int }; make() := bag(1); make().n", 1);
}
