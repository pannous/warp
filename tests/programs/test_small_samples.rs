// The small focused samples of the example menu (cards text-samples, error-examples): each shows one idea with an
// obvious result
use crate::is;

#[test]
fn text_samples() {
	is!("samples/palindrome.warp", true);
	is!("samples/word_count.warp", 4);
	is!("samples/sort_words.warp", warp::texts(vec!["brown", "fox", "quick", "the"]));
	is!("samples/word_lengths.warp", warp::ints(vec![3, 5, 5, 3]));
	is!("samples/replace_word.warp", "I like coffee");
	is!("samples/find_text.warp", true);
}

#[test]
fn error_samples() {
	is!("samples/try_catch.warp", "cannot divide by zero");
	is!("samples/try_else.warp", 0);
	is!("samples/parse_number.warp", -1);
	is!("samples/raise_error.warp", "no result");
	is!("samples/assertions.warp", "all good");
}
