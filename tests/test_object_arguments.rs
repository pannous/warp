// Objects and maps as function arguments: field access on a parameter, class instances for class-typed parameters,
// and the argument checked against a class-typed parameter (todo.md, impl-types 2026-10-03)
use crate::common::fails_with;
use warp::is;

const PHOTO: &str = "class photo{width:int height:int}; ";
const PAGE: &str = "class page{width:int}; ";
const KEEP: &str = "keep(p:photo) := p.width; ";

fn with_photo(code: &str) -> String {
	format!("{PHOTO}{code}")
}

#[test]
fn a_parameter_reads_a_field_of_a_map_argument() {
	is!("measure(p) := p.width; measure({width:3})", 3);
	is!("area(r) := r.wide * r.high; area({wide:2 high:3})", 6);
	is!("measure(p) := p.width; q = {width:4}; measure(q)", 4);
	is!("measure := it.width; measure({width:5})", 5);
}

#[test]
fn a_missing_field_of_a_map_argument_is_an_error() {
	fails_with("measure(p) := p.width; measure({height:3})", "width");
}

#[test]
fn a_class_instance_is_passed_to_a_class_typed_parameter() {
	is!(&with_photo(&format!("{KEEP}keep(photo{{width:3 height:4}})")), 3);
	is!(&with_photo("area(p:photo) := p.width * p.height; area(photo{width:3 height:4})"), 12);
	is!(&with_photo(&format!("{KEEP}x = photo{{width:5 height:1}}; keep(x)")), 5);
}

#[test]
fn a_class_instance_is_passed_to_an_untyped_parameter() {
	is!(&with_photo("measure(p) := p.height; measure(photo{width:3 height:4})"), 4);
}

#[test]
fn the_to_phrase_types_its_parameter_by_the_class() {
	is!(&with_photo("to keep a photo: photo.width; keep(photo{width:3 height:4})"), 3);
}

#[test]
fn a_number_for_a_class_typed_parameter_is_a_type_error() {
	fails_with(&with_photo(&format!("{KEEP}keep 3")), "photo");
	fails_with(&with_photo(&format!("{KEEP}keep(3)")), "photo");
}

#[test]
fn an_instance_of_another_class_is_a_type_error() {
	fails_with(&format!("{PHOTO}{PAGE}{KEEP}keep(page{{width:3}})"), "photo");
}
