//! A class from a used file: its methods work like a class of the program (modules::insert_module_classes)
use crate::is;

#[test]
fn a_used_modules_class_has_its_methods() {
	is!("tests/wasp/module_class/main.wasp", 41);
}

#[test]
fn the_programs_own_class_wins() {
	is!("tests/wasp/module_class/own.wasp", 7);
}
