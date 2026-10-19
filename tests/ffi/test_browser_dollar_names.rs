//! `$Math`, `$document` and `$xyz.member` of card browser-interaction: since card page-ids (user 2026-10-15) the old
//! forms keep working one release, with a hint naming the new one (`document.title`, `element("xyz").innerHTML`)
use crate::is;

#[test]
fn a_dollar_global_is_used_from_javascript() {
	is!("$Math.floor(2.7)", 2);
}

#[test]
fn a_dollar_name_is_the_element_of_that_id() {
	let code = "element(\"xyz\").innerHTML = \"x\"";
	warp::wasm_emitter::compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}"));
	crate::common::fails_with("$xyz.innerHtml = \"x\"", "did you mean innerHTML");
	crate::common::fails_with("document.titel", "did you mean title");
}

// the browser suite runs programs in a Worker: the page answers its calls of document (host-foreign.js callOnPage)
#[cfg(not(feature = "native"))]
#[test]
fn a_worker_reaches_the_pages_document() {
	is!("document.title", "warp tests in the browser"); // web/playground/tests.html
}

// card js-binding: `$` reads a page id's value; of a name that is a value already it is superfluous, warned and dropped
#[test]
fn a_superfluous_dollar_keeps_the_value_with_a_warning() {
	crate::common::warns_with("$Math.floor(2.7)", 2, "$Math is superfluous");
	crate::common::warns_with("x = 3; $x", 3, "$x is superfluous");
	crate::common::warns_with("f(n) := $n + 1; f(2)", 3, "$n is superfluous");
	let (_, warnings) = crate::common::warnings_of("$document.location");
	assert!(warnings.iter().any(|warning| warning.contains("$document is superfluous")), "{warnings:?}");
}
