//! `tree replace pattern with replacement` rewrites every match at any depth, children first, in data and in code
//! (card tree-replace, tree-patterns step 5)
use crate::is;

#[test]
fn replace_rewrites_list_matches_at_any_depth() {
	is!("xs = [[1, 2], [3, [1, 5]]]\nys = xs replace [1, x] with x\nys#1 + ys#2#2", 7);
}

#[test]
fn replace_rewrites_maps_inside_lists() {
	is!(
		"page = {top: {type: \"image\", src: \"logo.png\"}, body: [{type: \"text\"}, {type: \"image\", src: \"cat.jpg\"}]}\nlinked = page replace {type: \"image\", src} with {type: \"link\", href: src}\ncount(linked find {type: \"link\", href})",
		2
	);
}

#[test]
fn replace_keeps_what_does_not_match() {
	is!("xs = [1, [2, 3]]\nys = xs replace [9, x] with x\nys#2#2", 3);
}

#[test]
fn replace_simplifies_code_children_first() {
	is!("simplify(code) := code replace data($a * 1) with a\nstr(simplify(data (n * 1 + k * 1)))", "n+k");
}

#[test]
fn replace_takes_a_named_pattern() {
	is!("pattern Pair = [x, y]\nxs = [[1, 2], [3, 4], [5, 6]]\nys = xs replace Pair with x + y\nys#1 + ys#2", 10);
}
