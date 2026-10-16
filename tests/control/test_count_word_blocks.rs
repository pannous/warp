// card thrice-block (user): `thrice{print "hi"}` printed hi once and gave the tag thrice:ø. once, twice and thrice
// before a block repeat it like `3 times {…}`; a program that gives the word its own meaning keeps it
use crate::is;

#[test]
fn count_words_repeat_their_block() {
	is!("n=0; thrice{n+=1}; n", 3);
	is!("n=0; thrice {n+=1}; n", 3);
	is!("n=0; twice{n+=10}; n", 20);
	is!("n=0; once{n+=1}; n", 1);
	is!("n=0; if yes { twice{n+=1} }; n", 2);
}

#[test]
fn a_program_naming_the_word_keeps_its_meaning() {
	is!("twice(x) := x*2; twice(4)", 8);
	is!("twice := it*2; twice 5", 10);
}

#[test]
fn a_field_named_like_a_count_word_stays_data() {
	is!("p = {once: {a: 1}}; p.once.a", 1);
	is!("x = {twice: {a: 1}}; n = 0; thrice{n+=1}; n", 3);
}
