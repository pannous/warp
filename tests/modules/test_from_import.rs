//! `from list import zip, unique` brings only the named words of a module (card std-import)
use crate::common::fails_with;
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn from_import_brings_the_named_words() {
	is!("from list import zip; zip([1], [2])", parse("[[1 2]]"));
	is!("from list import zip, unique; unique([1, 2, 1])", parse("[1 2]"));
	is!("from text import repeat; repeat(\"a\", 2)", "aa");
}

#[test]
fn from_import_leaves_the_other_words_out() {
	fails_with("from list import zip; unique([1, 1])", "unique is in the standard module list");
}

#[test]
fn from_import_of_a_missing_word_is_an_error() {
	fails_with("from list import zap; 1", "list has no zap");
}

#[test]
fn a_program_word_of_an_unimported_name_is_its_own() {
	is!("from list import zip; unique(x) := 7; unique(1)", 7);
}
