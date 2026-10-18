//! card words-glue, decided plain since (decisions.md markup-plain-library): `;` ends an attribute's value before the
//! element's content, `label{ for: name; "Name" }` is for="name" with the text Name
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_text_after_a_value_is_content() {
	assert_eq!(html_of("label{ for: missing; \"L\" }"), "<label for=\"missing\">L</label>");
	assert_eq!(html_of("label{ for: email; \"Email\" b{ \"!\" } }"), "<label for=\"email\">Email<b>!</b></label>");
}

#[test]
fn a_bound_value_ends_before_a_text() {
	assert_eq!(
		html_of("name = \"a\"\nbutton{ bind: name; \"x\" }"),
		"<button value=\"a\" data-warp-input=\"1\">x</button>"
	);
}

#[test]
fn a_text_value_ends_before_a_text() {
	assert_eq!(html_of("p{ title: \"a b\"; \"hi\" }"), "<p title=\"a b\">hi</p>");
}
