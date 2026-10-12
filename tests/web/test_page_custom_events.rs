//! Card page-custom-event (notes/signals.md phase 7): in a page, an `emit` no handler of the program receives reaches
//! the page as a DOM CustomEvent (its name, the data as `detail`), so the page's own script can listen to the program;
//! a plain program's unhandled emit still does nothing (P202)
#[cfg(feature = "native")]
use warp::headless::Page;

#[test]
#[cfg(feature = "native")]
fn an_unhandled_emit_reaches_the_page() {
	let mut page = Page::render("div{ button{ on click { emit saved{id: 7} } \"Save\" } }").expect("renders");
	assert!(page.dispatched().is_empty());
	page.click("Save").expect("clicks");
	assert_eq!(page.dispatched(), [("saved".to_string(), "id:7".to_string())]);
}

#[test]
#[cfg(feature = "native")]
fn a_handled_emit_stays_in_the_program() {
	let code = "n = 0\non saved { n += 1 }\ndiv{ button{ on click { emit saved } \"Save\" } p{ n } }";
	let mut page = Page::render(code).expect("renders");
	page.click("Save").expect("clicks");
	assert!(page.dispatched().is_empty());
	assert_eq!(page.text(), "Save1");
}

#[test]
fn a_plain_program_s_unhandled_emit_does_nothing() {
	crate::is!("emit saved{id: 7}; 3", 3);
}
