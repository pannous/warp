//! Card web-fine-text-nodes (notes/web_framework.md step 3): a computed text between an element's children is a hole of
//! its own, `page·hole·<path>·text·<n>` (n the children before it), so a change reads and patches only that text node
//! instead of the element with its fixed children
use warp::warp_parser::parse;
use warp::web::markup::{holes, text_holes};

/// Each text hole as (element path, children before it, its text as written)
fn text_hole_places(markup: &str) -> Vec<(Vec<usize>, usize, String)> {
	text_holes(&parse(markup)).into_iter().map(|hole| (hole.path, hole.after, hole.text.serialize())).collect()
}

#[test]
fn a_computed_text_between_children_is_a_text_hole() {
	let markup = "div{ section{ h1{ \"Title\" } \"count \" + n p{ \"fixed\" } } }";
	assert_eq!(text_hole_places(markup), vec![(vec![0], 1, "\"count \"+n".to_string())]);
	assert!(holes(&parse(markup)).is_empty(), "the section is no element hole any more");
	// one before the first child, one after the last
	assert_eq!(
		text_hole_places("div{ section{ \"a\" + n h1{ \"t\" } \"b\" + m } }")
			.into_iter()
			.map(|(path, after, _)| (path, after))
			.collect::<Vec<_>>(),
		vec![(vec![0], 0), (vec![0], 1)]
	);
}

#[test]
fn an_element_whose_text_is_not_alone_stays_an_element_hole() {
	for markup in [
		"div{ section{ h1{ \"t\" } \"a \" \"count \" + n } }", // beside a fixed text: one DOM text node
		"div{ section{ h1{ \"t\" } n } }",                     // n may be markup
		"div{ section{ class: c h1{ \"t\" } \"count \" + n } }", // a computed attribute
		"div{ section{ \"count \" + n } }",                    // no child elements: the element is as small
	] {
		assert!(text_hole_places(markup).is_empty(), "{markup}");
		assert_eq!(holes(&parse(markup)).len(), 1, "{markup}");
	}
}

#[test]
fn a_page_binds_each_text_hole() {
	let code = "n = 0\ndiv{ section{ button{ on click { n += 1 } \"add\" } \"count \" + n } }";
	let lowered = warp::pipeline::lower(code).expect("lowers").serialize();
	assert!(lowered.contains("(page·hole·0·text·1):="), "{lowered}");
}

// card web-fine-rest: the root's own computed text is a text hole too, its other holes stay (the root itself none)
#[test]
fn the_roots_own_text_is_a_text_hole() {
	let markup = "div{ h1{ \"Counter\" } \"clicked \" + n p{ m } }";
	assert_eq!(text_hole_places(markup), vec![(vec![], 1, "\"clicked \"+n".to_string())]);
	assert_eq!(holes(&parse(markup)).into_iter().map(|(path, _)| path).collect::<Vec<_>>(), vec![vec![1]]);
	let code = "n = 0\ndiv{ button{ on click { n += 1 } \"add\" } \"clicked \" + n }";
	let lowered = warp::pipeline::lower(code).expect("lowers").serialize();
	assert!(lowered.contains("(page·hole·text·1):="), "{lowered}");
}
