//! card web-apis: `use js` of a browser global is typed through WebIDL (lib/web.webidl, from w3c/webref) as `use c`
//! is through the C headers: a member the global's interface lacks, an attribute called, or a call with an argument
//! count no overload takes is a compile-time error, natively (node) and in the page alike
use crate::common::fails_with;
use crate::{is, eq};

#[test]
fn a_member_the_interface_lacks_is_an_error_with_its_near_miss() {
	fails_with("use js crypto; crypto.randomUuid()", "did you mean randomUUID");
	fails_with("use js performance; performance.measur(\"a\")", "did you mean measure");
}

#[test]
fn a_call_with_a_count_no_overload_takes_is_an_error() {
	fails_with("use js performance; performance.mark()", "mark(DOMString markName");
	fails_with("use js crypto; crypto.randomUUID(1)", "randomUUID()");
}

#[test]
fn an_attribute_is_read_not_called() {
	fails_with("use js navigator; navigator.userAgent()", "attribute");
}

#[test]
fn what_the_interface_declares_runs() {
	is!("use js crypto; count crypto.randomUUID()", 36);
	is!("use js console; console.assert(1 == 1, \"a\", \"b\"); 7", 7); // optional, then variadic
	is!("use js performance; performance.now() > 0", 1); // Performance, gathered from the partials of several specs
}

#[test]
fn a_global_without_webidl_stays_unchecked() {
	is!("use js Math; Math.max(1, 5)", 5); // ECMAScript, not WebIDL
}

// the playground and the browser suite run programs in a Worker: its globals are WorkerGlobalScope's
#[cfg(not(feature = "native"))]
#[test]
fn a_worker_has_no_local_storage_and_its_own_navigator() {
	fails_with("use js localStorage; localStorage.getItem(\"k\")", "local[k]");
	fails_with("use js navigator; navigator.clipboard", "WorkerNavigator");
}

// natively a program's globals are a page's (node mirrors them, a built site runs there)
#[cfg(feature = "native")]
#[test]
fn a_page_has_local_storage_and_the_clipboard() {
	assert!(warp::web_idl::check("localStorage", "getItem", Some(1)).is_ok());
	assert!(warp::web_idl::check("localStorage", "setItem", Some(1)).is_err_and(|error| error.contains("setItem(DOMString key, DOMString value)")));
	assert!(warp::web_idl::check("navigator", "clipboard", None).is_ok());
}

#[test]
fn a_global_is_a_namespace_or_an_attribute_of_the_scope_not_an_interface() {
	is!("use js navigator; navigator.hardwareConcurrency > 0", 1);
	assert!(warp::web_idl::check("Storage", "anything", None).is_ok()); // the constructor, no instance
}

// an attribute's value is typed by its interface: navigator.clipboard is a Clipboard, also held in a variable
#[cfg(feature = "native")] // a Worker's navigator has no clipboard
#[test]
fn an_attributes_value_is_typed_by_its_interface() {
	fails_with("use js navigator; navigator.clipboard.writeTxt(\"x\")", "navigator.clipboard (Clipboard in WebIDL) has no member writeTxt; did you mean writeText");
	fails_with("use js navigator; board = navigator.clipboard; board.writeText()", "writeText(DOMString data)");
}

// a declared text, bool, int or float is a warp value: warp's methods apply, a variable holds it
#[test]
fn a_declared_primitive_result_is_a_warp_value() {
	is!("use js crypto; count crypto.randomUUID().upper()", 36);
	is!("use js crypto; id = crypto.randomUUID(); count id", 36);
	is!("use js navigator; navigator.hardwareConcurrency + 0.5 > 1", 1);
	eq!(warp::web_idl::result_type("Crypto", "randomUUID", true), Some("text"));
	eq!(warp::web_idl::result_type("Navigator", "onLine", false), Some("bool"));
	eq!(warp::web_idl::result_type("Performance", "now", true), Some("float"));
	eq!(warp::web_idl::result_type("Storage", "getItem", true), None); // DOMString? may be ø
	eq!(warp::web_idl::result_type("Navigator", "clipboard", false), None); // an object
}

// the text of a foreign call is the text of its value, not of the call
#[test]
fn a_foreign_calls_value_as_text() {
	is!("use js JSON; (JSON.stringify(1) as text) + 1", "11");
}
