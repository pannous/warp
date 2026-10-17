//! card click-key: besides click, key, input and submit an element handles change (a field's value committed), focus
//! and blur: `input{ on change { saved = event.value } }`
use warp::web::headless::Page;

#[test]
fn a_committed_value_reaches_on_change() {
	let mut page = Page::render(
		"saved = \"none\"\ndiv{ input{ aria-label: \"name\" on change { saved = event.value } } p{ saved } }",
	)
	.expect("renders");
	page.change("name", "Bob").expect("the field handles change");
	assert!(page.text().contains("Bob"), "{}", page.html());
}

#[test]
fn focus_and_blur_reach_their_handlers() {
	let mut page = Page::render(
		"state = \"idle\"\ndiv{ input{ aria-label: \"name\" on focus { state = \"typing\" } on blur { state = \"left\" } } p{ state } }",
	)
	.expect("renders");
	page.focus("name").expect("the field handles focus");
	assert!(page.text().contains("typing"), "{}", page.html());
	page.blur("name").expect("the field handles blur");
	assert!(page.text().contains("left"), "{}", page.html());
}

#[test]
fn a_field_event_is_no_markup_mistake() {
	warp::errors::diagnostic::take_warnings();
	warp::wasm_emitter::eval("x = 0\ninput{ aria-label: \"n\" on change { x = 1 } on focus { x = 2 } }");
	let warnings = warp::errors::diagnostic::take_warnings();
	assert!(!warnings.iter().any(|warning| warning.message.contains("is no event")), "{warnings:?}");
}
