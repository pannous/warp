//! Types a used file declares (card types-back): its type aliases name their types in the program's annotations, and a
//! class field typed map holds a map
use crate::is;

#[test]
fn an_alias_of_a_used_file_names_its_type() {
	is!("use \"tests/modules/used_types.warp\"; greeting(n: Name): Name := \"hi \" + n; greeting(\"Al\")", "hi Al");
}

#[test]
fn a_field_typed_map_holds_a_map() {
	is!("class Scope { kinds: map = {} }; s = Scope(); s.kinds[\"a\"] = \"int\"; s.kinds[\"a\"]", "int");
	is!("use \"tests/modules/used_types.warp\"; s = Scope(); s.kinds[\"x\"] = \"text\"; #s.kinds", 1);
}

#[test]
fn an_optional_result_of_a_used_files_type() {
	is!("use \"tests/modules/used_types.warp\"\nfirst(x: int): Name? := if x > 0 then \"a\" else ø\nfirst(1)", "a");
}
