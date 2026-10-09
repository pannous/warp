//! card split-without: `s.split` with no separator splits at spaces (the user's samples/word_count.warp)
use crate::is;

#[test]
fn split_without_separator_splits_at_spaces() {
	is!("#\"the quick brown fox\".split", 4);
	is!("\"a b\".split()[1]", "b");
	is!("x = \"a b c\"; #x.split", 3);
	is!("#split(\"a b\")", 2);
}
