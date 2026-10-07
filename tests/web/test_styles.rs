//! Styles as wasp data (card web-styles, notes/web_framework.md step 7): `style: { color: theme padding: 8 }` on an
//! element is its inline style, `style{ ".card": { padding: 8 } }` a style sheet; numbers are pixels unless the
//! property has no unit, camelCase names are CSS's kebab-case, values may read variables (shown anew as any markup)
use warp::html::to_html;
use warp::wasm_emitter::eval;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn an_inline_style_is_a_block_of_properties() {
	assert_eq!(html_of("theme = \"red\"\ndiv{ style: { color: theme padding: 8 } \"x\" }"), "<div style=\"color: red; padding: 8px\">x</div>");
	assert_eq!(html_of("p{ style: { fontSize: 12 opacity: 0.5 zIndex: 2 } \"x\" }"), "<p style=\"font-size: 12px; opacity: 0.5; z-index: 2\">x</p>");
}

#[test]
fn a_style_sheet_is_a_block_of_rules() {
	let sheet = html_of("div{ style{ \".card\": { padding: 8 color: \"navy\" } \"ul > li\": { margin: 0 } } p{ class:\"card\" \"x\" } }");
	assert_eq!(sheet, "<div><style>.card { padding: 8px; color: navy } ul > li { margin: 0px }</style><p class=\"card\">x</p></div>");
}

// a component's style sheet styles only its own elements (card web-scoped): its root element names the component
// (data-wasp-scope) and each selector of its sheet is prefixed with that
#[test]
fn a_style_sheet_in_a_component_is_scoped_to_it() {
	let card = "def Card(title) = div{ style{ \"h2\": { color: \"navy\" } } h2{ title } }\n";
	let scoped = "<div data-wasp-scope=\"Card\"><style>[data-wasp-scope=\"Card\"] h2 { color: navy }</style><h2>A</h2></div>";
	assert_eq!(html_of(&format!("{card}section{{ Card(\"A\") h2{{\"outside\"}} }}")), format!("<section>{scoped}<h2>outside</h2></section>"));
}
