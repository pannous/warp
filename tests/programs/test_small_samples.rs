// The small focused samples of the example menu (cards text-samples, error-examples): each shows one idea with an
// obvious result
use crate::is;

#[test]
fn text_samples() {
	is!("samples/text/palindrome.warp", true);
	is!("samples/text/word_count.warp", 4);
	is!("samples/text/sort_words.warp", warp::texts(vec!["brown", "fox", "quick", "the"]));
	is!("samples/text/word_lengths.warp", warp::ints(vec![3, 5, 5, 3]));
	is!("samples/text/replace_word.warp", "I like coffee");
	is!("samples/text/find_text.warp", true);
}

#[test]
fn error_samples() {
	is!("samples/errors/try_catch.warp", "cannot divide by zero");
	is!("samples/errors/try_else.warp", 0);
	is!("samples/errors/parse_number.warp", -1);
	is!("samples/errors/raise_error.warp", "no result");
	is!("samples/errors/assertions.warp", "all good");
}
