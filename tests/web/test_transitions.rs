//! Transitions declared as data (card web-transitions, notes/web_framework.md step 15): `li{ transition: fade 200ms }`
//! is the attribute data-wasp-transition, by which the page animates the element in when it appears, out when it goes
//! and, in a keyed list, to its new place (markup.js, FLIP)
use warp::markup::to_html;
use warp::wasm_emitter::eval;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_transition_is_data_on_its_element() {
	assert_eq!(html_of("ul{ li{ transition: fade 200ms\n \"a\" } }"), "<ul><li data-wasp-transition=\"fade 200ms\">a</li></ul>");
	assert_eq!(html_of("p{ transition: scale 2s ease-out\n \"b\" }"), "<p data-wasp-transition=\"scale 2000ms ease-out\">b</p>");
	assert_eq!(html_of("p{ transition: slide\n \"c\" }"), "<p data-wasp-transition=\"slide\">c</p>");
	assert_eq!(html_of("p{ transition: \"fade 1s\"\n \"d\" }"), "<p data-wasp-transition=\"fade 1s\">d</p>");
}

#[test]
fn a_keyed_item_keeps_its_transition() {
	let html = html_of("todos = [{id:7 text:\"a\"}]\nul{ [li{ key: todo.id transition: fade todo.text } for todo in todos] }");
	assert_eq!(html, "<ul><li data-wasp-key=\"7\" data-wasp-transition=\"fade\">a</li></ul>");
}

// in a style, transition stays the CSS property
#[test]
fn a_style_transition_is_css() {
	assert_eq!(html_of("p{ style: { transition: \"opacity 1s\" } \"e\" }"), "<p style=\"transition: opacity 1s\">e</p>");
}

// the words of a transition end where its element's children begin
#[test]
fn a_transition_ends_before_the_children() {
	assert_eq!(html_of("p{ transition: fade 200ms \"a\" }"), "<p data-wasp-transition=\"fade 200ms\">a</p>");
}
