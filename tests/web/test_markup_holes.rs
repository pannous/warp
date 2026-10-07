//! Card web-fine-holes (notes/web_framework.md step 3): the elements of a page's markup that hold a computed text,
//! attribute or child are read on their own after a handler (`page·hole·<path>`, the element indices from the root),
//! and the page changes only the ones whose HTML differs
use warp::markup::holes;
use warp::wasp_parser::parse;

const TOGGLE: &str = "done = false\ntoggles = 0\ndiv{ button{ on click { done = not done; toggles += 1 } \"toggle\" } p{ class: done ? \"done\" : \"open\" \"state\" } ul{ li{ \"toggled \" + toggles + \" times\" } } }";

fn hole_paths(markup: &str) -> Vec<Vec<usize>> {
	holes(&parse(markup)).into_iter().map(|(path, _)| path).collect()
}

#[test]
fn the_elements_holding_computed_parts_are_the_holes() {
	assert_eq!(hole_paths("div{ button{ \"toggle\" } p{ class: done ? \"done\" : \"open\" \"state\" } ul{ li{ \"n \" + n } } }"), vec![vec![1], vec![2, 0]]);
	// a fixed list's elements count as children; an element holding none itself is no hole, its children may be
	assert_eq!(hole_paths("div{ [p{\"a\"} p{\"b\"}] section{ h1{\"t\"} p{ n } } }"), vec![vec![2, 1]]);
	// a hole holds its whole element, nested ones included
	assert_eq!(hole_paths("div{ p{ n b{ m } } }"), vec![vec![0]]);
}

#[test]
fn fixed_markup_or_a_computed_root_has_no_holes() {
	assert!(hole_paths("div{ p{ \"a\" } ul{ li{ \"b\" } } }").is_empty());
	assert!(hole_paths("div{ \"count \" + n p{ m } }").is_empty());
}

#[test]
fn a_page_binds_each_hole() {
	let lowered = warp::pipeline::lower(TOGGLE).expect("lowers").serialize();
	assert!(lowered.contains("(page·hole·1):={p{"), "{lowered}");
	assert!(lowered.contains("(page·hole·2·0):={li{"), "{lowered}");
}
