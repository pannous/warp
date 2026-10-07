//! Styles as wasp data (card web-styles, notes/web_framework.md step 7): `style: { color: theme padding: 8 }` on an
//! element is its inline style, `style{ ".card": { padding: 8 } }` a style sheet; numbers are pixels unless the
//! property has no unit, camelCase names are CSS's kebab-case, values may read variables (shown anew as any markup)
use warp::markup::to_html;
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
