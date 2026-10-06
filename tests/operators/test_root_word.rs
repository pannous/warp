// `root` is the word sqrt (word-choice rule, wiki/pipe.md `2|square|root`); a variable or function named root wins (P142)
// in a local scope; at the top level `root` is a soft keyword that cannot be redefined (P165b)
use crate::common::fails_with;
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
	fails_with("root = 5; root + 1", "root is a keyword");
	fails_with("root(x) := x + 1; root 4", "root is a keyword");
	is!("tree = {root: 3}; tree.root", 3);
}
