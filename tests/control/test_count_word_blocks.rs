// card thrice-block (user): `thrice{print "hi"}` printed hi once and gave the tag thrice:ø. A statement starting with
// once, twice or thrice is read as `1/2/3 times …` (warp_parser/control/loops.rs try_parse_count_word); keys, tags of
// `key: value` blocks and a binding of the word in scope keep their meaning
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn count_words_count_like_times() {
	is!("n=0; thrice{n+=1}; n", 3);
	is!("n=0; thrice {n+=1}; n", 3);
	is!("n=0; twice{n+=10}; n", 20);
	is!("n=0; once{n+=1}; n", 1);
	is!("n=0; if yes { twice{n+=1} }; n", 2);
	is!("n=0; for x in [1 2] { thrice {n+=1} }; n", 6);
	is!("p={thrice:1}; n=0; thrice{n+=1}; n", 3);
	is!("twice \"ab\"", "abab");
}

#[test]
fn keys_and_tags_named_like_count_words_stay_data() {
	for code in ["{twice: {a: 1}}", "def f() := {twice: {a: 1}}; f()", "x = {twice: {a: 1}}; x", "twice{a: 1}"] {
		assert_eq!(eval(code).serialize(), "twice:a:1", "{code}");
	}
	is!("p = {once: {a: 1}}; p.once.a", 1);
}

#[test]
fn a_binding_of_the_word_keeps_its_meaning() {
	is!("twice(x) := x*2; twice(4)", 8);
	is!("twice := it*2; twice 5", 10);
	is!("def f(thrice) := thrice + 1; f(2)", 3);
}
