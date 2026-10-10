//! use, require and import are one statement and include its whole-file sibling: every pass asks modules.rs
//! is_import_keyword instead of a list of its own (card cleanup-closed-lists)
use crate::is;

#[test]
fn every_use_keyword_brings_a_module() {
	is!("use math; square 3", 9);
	is!("require math; square 3", 9);
	is!("import math; square 3", 9);
}

#[test]
fn a_hashed_import_keyword_keeps_the_line_a_comment() {
	is!("#require math\n2", 2);
	is!("#include math\n2", 2);
	is!("#import math\n2", 2);
}

#[test]
fn an_import_statement_has_no_effect_of_its_own() {
	is!("require math\neffects of main", "Pure");
	is!("use math\nglobal g = 1\ng = 2\neffects of main", "State");
}
