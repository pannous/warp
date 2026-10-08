//! Two-way bindings for forms (card web-bind, notes/web_framework.md step 6): `input{ bind: name }` shows name as its
//! value and sets name from what is typed (its handler of the page event input); a checkbox binds `checked`
use warp::markup::to_html;
use warp::wasm_emitter::eval;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_bound_input_shows_its_variable_and_handles_input() {
	assert_eq!(html_of("name = \"Ann\"\ndiv{ input{ bind: name } p{ \"Hi \" + name } }"), "<div><input value=\"Ann\" data-warp-input=\"1\"><p>Hi Ann</p></div>");
}

#[test]
fn a_bound_checkbox_is_checked_by_its_variable() {
	assert_eq!(html_of("done = true\ninput{ type: \"checkbox\" bind: done }"), "<input type=\"checkbox\" checked data-warp-input=\"1\">");
	assert_eq!(html_of("done = false\ninput{ type: \"checkbox\" bind: done }"), "<input type=\"checkbox\" data-warp-input=\"1\">");
}

#[test]
fn boolean_attributes_are_present_or_absent() {
	assert_eq!(html_of("button{ disabled: true \"Go\" }"), "<button disabled>Go</button>");
	assert_eq!(html_of("button{ disabled: false \"Go\" }"), "<button>Go</button>");
}
