//! `$xyz` is the page's element of id xyz, `$document`, `$window` and any other browser global that global itself, each
//! with its `use js` implied (card browser-interaction)
use crate::is;

#[test]
fn a_dollar_global_is_used_from_javascript() {
	is!("$Math.floor(2.7)", 2);
}

#[cfg(feature = "native")] // as test_foreign_member_assignment: the browser suite's Worker has no document
#[test]
fn a_dollar_name_is_the_element_of_that_id() {
	let code = "$xyz.innerHTML = \"x\"";
	warp::wasm_emitter::compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}"));
	crate::common::fails_with("$xyz.innerHtml = \"x\"", "did you mean innerHTML");
	crate::common::fails_with("$document.titel", "did you mean title");
}
