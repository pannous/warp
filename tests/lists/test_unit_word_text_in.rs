use crate::is;

// card in-text-word: a text naming a counting unit ("chars", "bytes") is a text before `in`, not the unit of
// `number of chars in t`

#[test]
fn a_text_naming_a_unit_is_looked_up_in_a_list() {
	is!("\"chars\" in [\"a\" \"chars\"]", true);
	is!("\"chars\" in [\"a\" \"b\"]", false);
	is!("\"bytes\" in [\"bytes\"]", true);
}

#[test]
fn a_unit_symbol_still_counts() {
	is!("number of chars in \"ab\"", 2);
	is!("t = \"héllo\"; number of bytes in t", 6);
}
