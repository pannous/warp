//! The standard library module list (notes/stdlib.md): `use list` loads std/list.wasp, embedded in warp
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn use_list_brings_its_words() {
	is!("use list; unique([1, 2, 1, 3, 2])", parse("[1 2 3]"));
	is!("use list; zip([1, 2, 3], [4, 5])", parse("[[1 4] [2 5]]"));
	is!("use list; enumerate([5, 6])", parse("[[1 5] [2 6]]"));
	is!("use list; product([2, 3, 4])", 24);
	is!("use list; mean([1, 2, 3, 4])", 2.5);
	is!("use list; take([1, 2, 3], 2)", parse("[1 2]"));
	is!("use list; drop([1, 2, 3], 1)", parse("[2 3]"));
	is!("use list; flatten([[1, 2], [3]])", parse("[1 2 3]"));
}

#[test]
fn a_programs_own_word_wins_over_the_module() {
	is!("use list; product(a, b) := a * b; product(3, 4)", 12);
}
