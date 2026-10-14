// Library words (`reverse`, `upper`, `sorted`, `count … in`) inside larger phrases: after an operator, as the element
// of a comprehension, and `where` as the filter of a comprehension
use crate::is;

#[test]
fn a_library_word_after_an_operator_takes_its_argument() {
	is!("word = \"level\"; word == reverse word", true);
	is!("word = \"hello\"; word == reverse word", false);
	is!("x = \"ab\"; \"BA\" == upper reverse x", true);
	is!("xs = [3, 1, 2]; 6 == sum xs", true);
}

#[test]
fn a_prefix_word_is_the_element_of_a_comprehension() {
	is!("words = [\"ab\", \"cd\"]; join([upper w for w in words], \" \")", "AB CD");
	is!("s = \"banana\"; vowels = ['a', 'e']; [count v in s for v in vowels]#1", 3);
	is!("s = \"banana\"; vowels = ['a', 'e']; [count v in s for v in vowels]#2", 0);
}

#[test]
fn where_filters_a_comprehension_like_if() {
	is!("count([c for c in \"banana\" where c == 'a'])", 3);
	is!("[x * 10 for x in [1, 2, 3] where x > 1]#1", 20);
}

#[test]
fn sorted_text_sorts_its_characters() {
	is!("sorted \"hello\"", "ehllo");
	is!("sort(\"cab\")", "abc");
	is!("word = sorted \"listen\"; word == sorted \"silent\"", true);
}

#[test]
fn a_suffix_word_after_a_library_word_is_its_prefix_argument() {
	is!("xs = [3, 1, 2]; first sorted xs", 1);
	is!("xs = [3, 1, 2]; last sorted xs", 3);
	is!("xs = [3, 1, 2]; first sort xs", 1);
	is!("xs = [3, 1, 2]; (xs sorted)#1", 1);
}

#[test]
fn a_statement_level_library_word_takes_one_operand() {
	is!("sorted \"listen\" == sorted \"silent\"", true);
	is!("reverse \"ab\" == \"ba\"", true);
	is!("xs = [1, 2]; sum xs == 3", true);
}
