//! A declared list holds only items of its element type, checked like a declared scalar (card list-element-types)
use crate::common::fails_with;
use crate::is;

const DECLARED: &str = "names is declared texts";

#[test]
fn declared_lists_keep_fitting_items() {
	is!("names: texts = [\"hi\" \"yo\"]; names.add(\"ok\"); #names", 3);
	is!("names: list of text = [\"hi\"]; names = names + [\"yo\"]; #names", 2);
	is!("xs: numbers = [1 2.5]; xs.add(3); #xs", 3);
	is!("xs: floats = [1]; xs#1", 1);
	is!("names: texts = []; names.add(\"a\"); #names", 1);
}

#[test]
fn declared_list_refuses_a_literal_of_another_type() {
	fails_with("names: texts = [420]", DECLARED);
	fails_with("names: texts = [\"hi\" 420]", DECLARED);
	fails_with("names: list of text = [420]", "names is declared list of text");
	fails_with("xs: ints = [1.5]", "xs is declared ints");
}

#[test]
fn declared_list_refuses_another_type_later() {
	fails_with("names: texts = [\"hi\"]; names.add(420); names", DECLARED);
	fails_with("names: texts = [\"hi\"]; names = names + [420]; names", DECLARED);
	fails_with("names: texts = [\"hi\"]; names = [420]; names", DECLARED);
}

#[test]
fn declared_list_checks_run_time_items_at_the_store() {
	fails_with("same(y) := y; names: texts = [\"hi\"]; names.add(same(420)); names", DECLARED);
	fails_with("numbers() := [1 2]; names: texts = numbers(); names", DECLARED);
	is!("same(y) := y; names: texts = [\"hi\"]; names.add(same(\"yo\")); #names", 2);
}

#[test]
fn list_fields_check_their_items() {
	fails_with("class bag { items: texts }; bag([420])", "bag.items is texts");
	fails_with("class bag { items: texts }; b = bag([\"hi\"]); b.items.add(420); b.items", "items of bag");
	is!("class bag { items: texts }; b = bag([\"hi\"]); b.items.add(\"yo\"); #b.items", 2);
}

#[test]
fn declared_list_checks_element_assignment() {
	fails_with("names: texts = [\"hi\"]; names#1 = 420; names", DECLARED);
	fails_with("same(y) := y; names: texts = [\"hi\"]; names#1 = same(420); names", DECLARED);
	is!("names: texts = [\"hi\"]; names#1 = \"yo\"; names#1", "yo");
	is!("xs: ints = [1 2]; xs#2 = 5; xs#2", 5);
}

#[test]
fn list_fields_check_run_time_items() {
	let program = "same(y) := y; class bag { items: texts }; b = bag([\"hi\"]); ";
	fails_with(&format!("{program}b.items.add(same(420)); b.items"), "items of bag is declared texts");
	is!(&format!("{program}b.items.add(same(\"yo\")); #b.items"), 2);
}

#[test]
fn declared_list_checks_the_items_of_another_list() {
	let numbers = "numbers() := [1 2]; other = numbers(); names: texts = [\"hi\"]; ";
	fails_with(&format!("{numbers}names = other + [\"yo\"]; names"), DECLARED);
	fails_with("numbers() := [1 2]; names: texts = numbers() + [\"yo\"]; names", DECLARED);
	is!("words() := [\"a\" \"b\"]; names: texts = words() + [\"c\"]; #names", 3);
	is!("names: texts = [\"hi\"]; more: texts = [\"yo\"]; names = more + [\"ok\"]; #names", 2);
}
