//! `tree find pattern` matches the pattern at any depth of a tree of maps, pairs and lists (XPath `//`, jq `..`): the
//! values of its one name, or the matching subtrees when it binds none (card tree-find, tree-patterns step 5)
use crate::is;

const PAGE: &str = "page = {top: {logo: {type: \"image\", src: \"logo.png\"}}, body: [{type: \"text\"}, {type: \"image\", src: \"cat.jpg\"}]}\n";

#[test]
fn find_collects_a_name_at_any_depth() {
	is!(&format!("{PAGE}sources = page find {{type: \"image\", src}}\nsources#1"), "logo.png");
	is!(&format!("{PAGE}sources = page find {{type: \"image\", src}}\nsources#2"), "cat.jpg");
}

#[test]
fn find_without_names_gives_the_subtrees() {
	is!(&format!("{PAGE}count(page find {{type: \"text\"}})"), 1);
}

#[test]
fn find_takes_a_named_pattern() {
	is!(&format!("{PAGE}pattern Image = {{type: \"image\", src}}\ncount(page find Image)"), 2);
}

#[test]
fn find_matches_list_patterns_in_nested_lists() {
	is!("xs = [[1, 2], [3, [1, 5]]]\nfound = xs find [1, x]\nfound#1 + found#2", 7);
}

#[test]
fn two_finds_walk_apart() {
	is!(&format!("{PAGE}count(page find {{type: \"text\"}}) + count(page find {{src}})"), 3);
}
