//! `receiver.member = value` of a foreign receiver sets the member in its runtime: a module, a variable holding a
//! handle, or a chained call (card js-member: `document.getElementById("r").innerHTML = "x"`)
use crate::is;

#[test]
fn a_js_member_is_set() {
	is!("use js Math; Math.answer = 42; Math.answer", 42);
	is!("use js Map; m = Map(); m.answer = \"yes\"; m.answer", "yes");
}

#[test]
fn a_member_of_a_call_result_is_set() {
	is!("use js Map; m = Map(); m.set(\"k\", Map()); m.get(\"k\").answer = 3; m.get(\"k\").answer", 3);
}

#[cfg(feature = "native")] // as tests/web/test_web.rs: the browser suite's Worker has no document
#[test]
fn a_page_element_member_assignment_compiles() {
	let code = "use js document; document.getElementById(\"r\").innerHTML = \"x\"";
	warp::wasm_emitter::compile(code).unwrap_or_else(|error| panic!("{code} does not compile: {error:?}"));
	crate::common::fails_with("use js document; document.getElementById(\"r\").innerHtml = \"x\"", "did you mean innerHTML");
}

#[cfg(feature = "native")] // python3
#[test]
fn a_python_attribute_is_set() {
	is!("use python \"types\"; n = types.SimpleNamespace(); n.answer = 4; n.answer", 4);
}
