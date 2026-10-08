//! lib/prelude.wasp (card std-prelude-module, notes/stdlib.md §8): the wasp-written words every program has without
//! `use`, formerly templates in the compiler; spellings and method forms still reach them
use crate::is;
use warp::wasp_parser::parse;

#[test]
fn the_prelude_words_work_without_use() {
	is!("first([3, 4])", 3);
	is!("last(\"abc\")", 'c');
	is!("round(2.71828, 2)", 2.72);
	is!("replace(\"a-b\", \"-\", \"+\")", "a+b");
	is!("[is_digit('5'), is_alpha('x'), is_alphanumeric('_')]", parse("[yes yes no]"));
}

#[test]
fn methods_and_other_spellings_reach_the_prelude_words() {
	is!("\"ab\".first()", 'a');
	is!("[1, 2, 3].last", 3);
	is!("isdigit('7')", true);
	is!("'q'.isalpha()", true);
	is!("x = 2.71828; x.round(1)", 2.7);
}

#[test]
fn a_module_word_inside_an_interpolation_comes_along() {
	is!("o = 2.71828; \"a \\(o.round(1)) b\"", "a 2.7 b");
	is!("use list; \"\\(zip([1], [2]))\"", "[[1 2]]");
}

#[test]
fn a_local_named_like_a_prelude_word_is_the_local() {
	is!("def pop1(items) { first = items#1; [first, last(items)] }; pop1([4, 5])", parse("[4 5]"));
}

#[test]
fn a_programs_own_prelude_word_wins() {
	is!("first(x) := 7; first([1])", 7);
}

#[test]
fn dir_lists_the_prelude() {
	is!("dir(prelude)#1", "first");
}
