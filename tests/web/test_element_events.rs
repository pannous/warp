//! `button{ on click { count += 1 } "Add" }`: a handler on an element (card web-element, notes/web_framework.md step 2).
//! The element carries data-warp-click with its handler's number; the page calls that handler on a click inside it and
//! shows the program's markup anew
use warp::markup::to_html;
use warp::wasm_emitter::eval;

const COUNTER: &str = "count = 0\ndiv{ button{ on click { count += 1 } \"Add\" } p{ \"clicked \" + count } }";

#[test]
fn an_element_handler_marks_its_element() {
	assert_eq!(to_html(&eval(COUNTER)), "<div><button data-warp-click=\"1\">Add</button><p>clicked 0</p></div>");
}

#[test]
fn each_element_handler_has_its_number() {
	let html = to_html(&eval("n = 0\ndiv{ button{ on click { n += 1 } \"+\" } button{ on click { n -= 1 } \"-\" } p{ n } }"));
	assert_eq!(html, "<div><button data-warp-click=\"1\">+</button><button data-warp-click=\"2\">-</button><p>0</p></div>");
}

#[test]
fn the_page_shows_the_markup_after_a_handler() {
	let shown = warp::web::shown(&serde_json::json!({"result": {"kind": "1", "data": {"int": "5"}, "chain": []}}));
	assert_eq!(shown["value"], "5");
	assert!(shown["html"].is_null());
}
