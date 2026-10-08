// cards approximately and approximately-all (notes/approximately.md): `≈` (`~`, circa, approximately) holds for any two
// values: numbers within the relative `tolerance`, texts alike in case and accents, a bool and any value alike in
// truthiness, lists, objects and instances field by field with ≈ again
use crate::is;

#[test]
fn instances_are_approximately_equal_field_by_field() {
	is!("class Point{x:float y:float}; Point(1, 2) ≈ Point(1.0000000001, 2)", true);
	is!("class Point{x:float y:float}; Point(1, 2) ≈ Point(1.1, 2)", false);
}

#[test]
fn lists_and_objects_are_approximately_equal_element_by_element() {
	is!("[1 2] ≈ [1.0000000001 2]", true);
	is!("[1 2] ≈ [1 2 3]", false);
	is!("{x:1 y:2} ≈ {y:2.0000000001 x:1}", true);
	is!("{p:[1 2]} ≈ {p:[1.0000000001 2]}", true);
	is!("tolerance = 0.01; [100] ≈ [100.5]", true);
}

#[test]
fn texts_are_approximately_equal_in_any_case_and_accent() {
	is!("\"hí\" ≈ \"HI\"", true);
	is!("\"a\" ≈ \"b\"", false);
}

#[test]
fn a_bool_is_approximately_any_value_of_its_truthiness() {
	is!("yes ≈ 2", true);
	is!("yes ≈ 0", false);
	is!("no ≈ \"\"", true);
}
