//! `use js Math`: a JavaScript global through foreign_call, node natively, the page itself in the browser host
//! (web/playground/host.js); modules (`use js "path"`) natively only (notes/stdlib_connectors.md)
use crate::common::fails_with;
use warp::is;

#[test]
fn test_js_globals() {
	is!("use js Math; Math.max(1, 5)", 5);
	is!("use js Math; Math.max(1, 5) + 1", 6);
	is!("use js \"Math\"; Math.PI > 3", 1);
	is!("use js JSON; JSON.stringify([1, 2])", "[1,2]");
}

#[test]
fn test_an_unknown_js_member_is_loud() {
	fails_with("use js Math; Math.nope(1)", "Math has no nope");
}

#[cfg(feature = "native")] // node's modules
#[test]
fn test_js_modules() {
	is!("use js \"path\"; path.join(\"a\", \"b\")", "a/b");
	is!("use js \"crypto\"; count crypto.randomUUID()", 36);
	is!("use python math; use js Math; math.pi - Math.PI", 0);
}

#[test]
fn test_values_held_from_another_runtime_compare() {
	is!("use js Math; if Math.max(1.5, 0) > 1 {1} else {0}", 1);
}

#[test]
fn test_untrusted_code_gets_no_js() {
	let result = warp::pipeline::eval_untrusted("use js Math; Math.max(1, 5)");
	assert!(result.serialize().contains("capability"), "{result:?}");
}
