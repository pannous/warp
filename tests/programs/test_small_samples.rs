// The small focused samples of the example menu (cards text-samples, error-examples): each shows one idea with an
// obvious result
use crate::is;

#[test]
fn text_samples() {
	is!("samples/palindrome.wasp", true);
	is!("samples/word_count.wasp", 4);
	is!("samples/sort_words.wasp", warp::texts(vec!["brown", "fox", "quick", "the"]));
	is!("samples/word_lengths.wasp", warp::ints(vec![3, 5, 5, 3]));
	is!("samples/replace_word.wasp", "I like coffee");
	is!("samples/find_text.wasp", true);
}

#[test]
fn error_samples() {
	is!("samples/try_catch.wasp", "cannot divide by zero");
	is!("samples/try_else.wasp", 0);
	is!("samples/parse_number.wasp", -1);
	is!("samples/raise_error.wasp", "no result");
	is!("samples/assertions.wasp", "all good");
}
