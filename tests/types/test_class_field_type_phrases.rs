// A field type written in words, `items: list of text`, is one type like `items: texts` and `items: list<text>`
use crate::is;

#[test]
fn a_field_type_of_words_is_one_field() {
	is!("class bag { items: list of text }; b = bag([\"hi\"]); count(b.items)", 1);
	is!("class bag { items: list of text; n: int }; b = bag([\"hi\"], 2); b.n", 2);
	is!("class bag { items: list of list of int }; b = bag([[1, 2]]); b.items#1#2", 2);
	is!("class bag { items: list<text> }; count(bag([\"hi\", \"yo\"]).items)", 2);
}
