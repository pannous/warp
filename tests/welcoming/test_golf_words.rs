// cards golf-chr, golf-chars, golf-print, golf-inline, and `for char in text`: short forms code golf reaches for
use crate::is;
use warp::*;

#[test]
fn chr_of_an_expression_is_a_function_result() {
	is!("f(x) := chr(x + 1); f(66)", 'C');
	is!("b=97; rot(c) := chr((ord(c) - b + 13) % 26 + b); rot('a')", 'n');
}

#[test]
fn walked_chars_are_the_list_of_chars() {
	is!("\"ab\".chars", texts(vec!["a", "b"])); // the list as everywhere (user 2026-10-14, card abc-chars)
	is!("w=\"ab\"; w.chars.map(c => ord c)", ints![97, 98]);
	is!("s=0; for c in \"ab\".chars { s += ord c }; s", 195);
}

#[test]
fn a_char_loop_over_a_text_visits_every_char() {
	is!("n=0; for char in \"abc\" { n += 1 }; n", 3);
	is!("s=\"abc\"; n=0; for char in s { n += 1 }; n", 3);
	is!("s=\"abc\"; s#1 is char", true);
	is!("s=\"abc\"; s is char", false);
	is!("n=0; for int in \"abc\" { n += 1 }; n", 0);
}

#[cfg(feature = "native")]
#[test]
fn print_without_parentheses_in_an_operand_or_branch() {
	use crate::common::printed;
	assert_eq!(printed("for 1…5:it%2&&print it"), "1\n3\n5\n");
	assert_eq!(printed("for 1…5:if it%2:print it"), "1\n3\n5\n");
	assert_eq!(printed("for 1…5:print it if it%2"), "1\n3\n5\n");
	assert_eq!(printed("x=4; if x%2: print x else print x+1"), "5\n");
	assert_eq!(printed("for c in \"ab\": print ord c"), "97\n98\n");
	assert_eq!(printed("for char in \"ab\": print it"), "a\nb\n");
}

#[test]
fn bits_of_whole_numbers() {
	is!("use math; bit_count(7)", 3);
	is!("use math; bit_count(0)", 0);
	is!("use math; bit_and(12, 10)", 8);
	is!("use math; bit_or(12, 10)", 14);
}

#[test]
fn a_text_word_without_its_module_is_a_text_method() {
	is!("use math; to_base(5, 2).count_of(\"1\")", 2);
}

#[cfg(feature = "native")]
#[test]
fn a_bare_for_walks_a_method_chain() {
	use crate::common::printed;
	assert_eq!(printed("for (1…5).filter(x=>x%2):print it"), "1\n3\n5\n");
	assert_eq!(printed("for (1…3) {print it}"), "1\n2\n3\n");
	assert_eq!(printed("for(i=0;i<2;i++){print i}"), "0\n1\n");
}

/// card two-ints: bit_and and bit_or take two's-complement bits of any whole numbers, as a warp-written emitter needs
/// for LEB128 (`bit_and(n, 127)` of a negative n)
#[test]
fn bit_words_take_the_bits_of_negative_numbers_too() {
	is!("use math; bit_and(-129, 127)", 127);
	is!("use math; bit_and(-1, 255)", 255);
	is!("use math; bit_or(-128, 1)", -127);
	is!("use math; bit_and(624485, 127)", 101);
	is!("use math; bit_or(101, 128)", 229);
}
