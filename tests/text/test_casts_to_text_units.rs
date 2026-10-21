//! card selector-casts (decision text-unit-plurals): a cast to a text unit word is that list, `t as words` is
//! `t.words`, for every unit word of the one table; a list cast to an element type still casts each item
use crate::is;

#[test]
fn a_text_cast_to_a_unit_word_is_its_list() {
	is!("#(\"ab cd ef\" as words)", 3);
	is!("t = \"ab cd\\nef\"\n(t as lines)#2", "ef");
	is!("#(\"a1b\" as letters)", 2);
	is!("#(\"abc\" as glyphs)", 3);
	is!("#(\"abc\" as graphemes)", 3);
	is!("#(\"héllo\" as bytes)", 6);
	is!("(\"abc\" as chars)#3", 'c');
}

#[test]
fn a_list_cast_to_an_element_type_casts_each_item() {
	is!("([\"1\", \"2\"] as ints)#2", 2);
}
