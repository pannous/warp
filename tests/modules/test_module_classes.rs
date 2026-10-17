//! A class from a used file: its methods work like a class of the program (modules::insert_module_classes)
use crate::is;

#[test]
fn a_used_modules_class_has_its_methods() {
	is!("tests/warp/module_class/main.warp", 41);
}

#[test]
fn the_programs_own_class_wins() {
	is!("tests/warp/module_class/own.warp", 7);
}

#[test]
fn a_used_modules_instance_reads_a_field_named_like_a_word() {
	// `room.effects` reads the field, not the reflection word `f.effects` (card lib-field-name)
	is!("tests/warp/module_class/bus_fields.warp", 9);
}
