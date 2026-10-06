//! `==` compares by value: objects are maps (any entry order), lists element by element, numbers exactly (wiki/equality.md)

use crate::is;

#[test]
fn objects_compare_by_their_entries() {
	is!("{a:1 b:2} == {a:1 b:2}", 1);
	is!("{a:1} == {a:2}", 0);
	is!("{a:1} == {a:1}", 1);
	is!("{a:1 b:2} == {a:1}", 0);
}

#[test]
fn the_entry_order_of_an_object_does_not_matter() {
	is!("{a:1 b:2} == {b:2 a:1}", 1);
	is!("{a:1 b:2 c:3} == {c:3 a:1 b:2}", 1);
	is!("{a:1 b:2} == {b:2 a:9}", 0);
}

#[test]
fn a_float_equals_the_same_exact_number() {
	is!("{x:3.0} == {x:3}", 1);
	is!("{x:{y:3.0}} == {x:{y:3}}", 1);
	is!("{x:0.5} == {x:0.5}", 1);
	is!("{x:0.5} == {x:0.25}", 0);
	is!("[1.5 2] == [1.5 2]", 1);
	is!("[1.5 2] == [1.25 2]", 0);
}

#[test]
fn lists_compare_element_by_element() {
	is!("[1 2] == [1 2]", 1);
	is!("[1 2] == [1 2 3]", 0);
	is!("[1 2] == [2 1]", 0);
	is!("[[1],[2]] == [[1],[2]]", 1);
	is!("[[1],[2]] == [[1],[3]]", 0);
}

#[test]
fn not_equal_is_the_negation() {
	is!("{a:1} != {a:1}", 0);
	is!("{a:1} != {a:2}", 1);
	is!("{a:1 b:2} != {b:2 a:1}", 0);
	is!("[1 2] != [1 2]", 0);
	is!("[1 2] != [1 3]", 1);
}

#[test]
fn variables_holding_structures_compare_by_value() {
	is!("p={a:1 b:2}; q={b:2 a:1}; p==q", 1);
	is!("xs=[1 2]; ys=[1 2]; xs==ys", 1);
}

#[test]
fn struct_instances_compare_by_their_fields() {
	is!("struct point{x:int y:int}; point(1,2) == point(1,2)", 1);
	is!("struct point{x:int y:int}; point(1,2) == point(1,3)", 0);
	is!("struct point{x:int y:int}; point(1,2) != point(1,3)", 1);
}

#[test]
fn a_text_never_equals_a_number() {
	is!("\"3\" == 3", 0);
	is!("[1 2] == 3", 0);
	is!("{a:1} == 1", 0);
}

#[test]
fn a_switch_chooses_a_case_by_the_same_equality() {
	is!("switch [1 2] {[1 2]: 7 default: 0}", 7);
	is!("switch [1 3] {[1 2]: 7 default: 0}", 0);
}
