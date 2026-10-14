//! `import square from math` and `include math` reach the embedded standard modules like `use math` and
//! `from math import square` do (cards import-square, include-math)
use crate::common::fails_with;
use crate::is;

#[test]
fn import_from_brings_the_named_words_of_a_standard_module() {
	is!("import square from math; square 4", 16);
	is!("import (square) from math; square 4", 16);
	is!("import (zip, unique) from list; unique([1, 2, 1])", warp::warp_parser::parse("[1 2]"));
}

#[test]
fn import_from_leaves_the_other_words_of_the_module_out() {
	fails_with("import zip from list; unique([1, 1])", "unique is in the standard module list");
}

#[test]
fn import_from_a_foreign_library_stays_the_foreign_import() {
	is!("import sqrt from math; sqrt 4", 2);
}

#[test]
fn include_splices_a_standard_module() {
	is!("include math; square 3", 9);
	is!("include list; zip([1], [2])", warp::warp_parser::parse("[[1 2]]"));
}
