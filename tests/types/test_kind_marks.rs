//! Card kind-reflection: lib words read a value's kind field (src/type_kinds.rs), its Kind, the info byte above it
//! and the type-value mark, as the lib printers (list_text, node_type_name, map keys) will
use crate::is;

#[test]
fn the_kind_number_is_the_low_byte() {
	is!("kind_number(3)", 1);
	is!("kind_number(sqrt(2.0))", 2);
	is!("kind_number(\"ab\")", 3);
	is!("kind_number([1 2])", 8);
	is!("kind_number(error(\"boom\"))", 11);
	is!("f(x: any) := kind_number(x); f(sqrt(2.0))", 2);
}

#[test]
fn the_kind_info_is_the_byte_above() {
	is!("kind_info(true)", 1);
	is!("kind_info(1)", 0);
	is!("kind_info([1 2])", 1);
}

#[test]
fn a_type_value_is_marked() {
	is!("is_type_value(int)", true);
	is!("is_type_value(type(\"a\"))", true);
	is!("is_type_value(symbol(\"int\"))", false);
	is!("is_type_value(3)", false);
}

#[test]
fn the_kind_mark_is_the_whole_field() {
	is!("kind_mark(true)", 257);
	is!("kind_mark([1 2]) % 256", 8);
}
