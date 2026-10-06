// `root` is the word sqrt (word-choice rule, wiki/pipe.md `2|square|root`); a variable or function named root wins (P142)
use crate::is;

#[test]
fn root_is_sqrt() {
	is!("root 4", 2);
	is!("root(9)", 3);
	is!("square x := x*x; 2|square|root", 2);
	is!("square x := x*x; square 2|root", 2);
}

#[test]
fn a_defined_root_wins() {
	is!("root = 5; root + 1", 6);
	is!("root(x) := x + 1; root 4", 5);
	is!("tree = {root: 3}; tree.root", 3);
}
