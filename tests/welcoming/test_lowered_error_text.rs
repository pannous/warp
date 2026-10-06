// An error quotes the program as written, never the compiler's rewrite of it (temporaries such as `range_value·item`)
use crate::common::fails_with;
use warp::wasm_emitter::eval;

#[test]
fn an_unknown_word_error_quotes_the_words_as_written() {
	for (code, written) in [("n=3; cube 1..n", "`cube 1..n`"), ("n=3; [cube 1..n]", "`cube 1..n`"), ("a=1; b=4; blub a..b; 2", "`blub a..b`")] {
		fails_with(code, written);
		let message = eval(code).serialize();
		assert!(!message.contains('·'), "{message}");
	}
}
