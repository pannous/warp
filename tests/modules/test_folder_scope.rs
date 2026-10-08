//! D15 (user, 2026-10-03, revised the same day): `use folder` makes all .warp files in the folder of the compiled file
//! visible, like a Go package (no longer the default, see test_use_scopes). A name is looked up in the siblings only
//! when the file does not define it; two siblings defining a used name is an error naming both files.
use crate::is;
use crate::common::fails_with;

#[test]
fn sibling_definitions_are_visible_without_use() {
	is!("tests/warp/folder_scope/main.warp", 29);
}

#[test]
fn the_file_itself_wins_over_its_siblings() {
	is!("tests/warp/folder_own/main.warp", 3);
}

#[test]
fn a_name_defined_in_two_siblings_is_ambiguous() {
	fails_with("tests/warp/folder_ambiguous/main.warp", "pick is defined in more than one file of the scope");
	fails_with("tests/warp/folder_ambiguous/main.warp", "a.warp");
	fails_with("tests/warp/folder_ambiguous/main.warp", "b.warp");
}

#[test]
fn inline_code_has_no_folder_scope() {
	fails_with("area(3)", "area");
}
