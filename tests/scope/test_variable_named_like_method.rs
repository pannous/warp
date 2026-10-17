// Card bytes-variable-clash: a variable named like a library word hides the word where the name stands alone, never after
// a dot: `t.bytes` with `bytes = …` still calls the word, as a variable is no member of the receiver
use crate::is;

#[test]
fn a_variable_named_like_a_word_leaves_the_word_after_a_dot() {
	is!("size(t: text) := #t.bytes\nbytes = [1, 2, 3]\nsize(\"main\")", 4);
	is!("shout(t: text) := t.upper\nupper = 3\nshout(\"hi\")", "HI");
	is!("bytes = [1]\n(\"ab\".bytes)#2", 98);
	is!("bytes = [1, 2]\n#bytes", 2);
}
