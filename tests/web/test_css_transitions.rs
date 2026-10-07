//! Transitions as real CSS (card web-css, user: stay close to HTML and CSS): `transition:` in an element is its inline
//! CSS transition, `starting-style: {…}` the state it enters from and leaves towards (CSS @starting-style, which an
//! inline style cannot hold, so the page applies it, markup-transitions.js); fade/scale/slide are this CSS, with a hint to it
use warp::markup::to_html;
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_css_transition_is_the_inline_style() {
	assert_eq!(html_of("p{ transition: opacity 200ms ease-out \"a\" }"), "<p style=\"transition: opacity 200ms ease-out\">a</p>");
	assert_eq!(html_of("p{ transition: \"opacity 200ms, transform 300ms\" \"a\" }"), "<p style=\"transition: opacity 200ms, transform 300ms\">a</p>");
}

#[test]
fn a_css_transition_joins_the_element_style() {
	assert_eq!(html_of("p{ style: { color: \"red\" } transition: opacity 1s \"a\" }"), "<p style=\"color: red; transition: opacity 1000ms\">a</p>");
	assert_eq!(html_of("p{ style: \"color: red\" transition: opacity 1s \"a\" }"), "<p style=\"color: red; transition: opacity 1000ms\">a</p>");
}

#[test]
fn the_starting_style_is_data_for_the_page() {
	let html = html_of("ul{ li{ starting-style: { opacity: 0 transform: \"scale(0.8)\" } transition: \"opacity 200ms, transform 200ms\" \"a\" } }");
	assert_eq!(html, "<ul><li data-wasp-starting-style=\"opacity: 0; transform: scale(0.8)\" style=\"transition: opacity 200ms, transform 200ms\">a</li></ul>");
}

#[test]
fn a_made_up_kind_is_css_with_a_hint() {
	let (shown, hints) = capture_hints(|| html_of("p{ transition: fade 200ms\n \"a\" }"));
	assert_eq!(shown, "<p data-wasp-starting-style=\"opacity: 0\" style=\"transition: opacity 200ms, transform 200ms\">a</p>", "fade still works");
	let canonical: Vec<String> = hints.into_iter().map(|hint| hint.canonical).collect();
	assert!(canonical.contains(&"transition: \"opacity 200ms, transform 200ms\" starting-style: { opacity: 0 }".to_string()), "{canonical:?}");
}

// a built site carries markup-transitions.js, after markup.js, only when its elements have transitions: the hello-world
// budget (test_bundle_budget) pays nothing for them
#[test]
#[cfg(feature = "native")] // src/site.rs is native
fn a_site_with_transitions_carries_their_script() {
	let scripts = |code: &str| {
		let directory = crate::common::scratch_directory("transitions-site");
		let site = warp::site::build(code, "list", &directory).expect("the site is built");
		std::fs::remove_dir_all(directory).unwrap();
		site.files
	};
	let with = scripts("div{ ul{ li{ key: 1 starting-style: { opacity: 0 } transition: opacity 150ms \"milk\" } } }");
	assert_eq!(with[with.len() - 3..], ["markup.js", "markup-transitions.js", "site.js"]);
	assert!(!scripts("div{ p{ \"hello\" } }").contains(&"markup-transitions.js".to_string()));
}
