//! card web-apis: `use js` of a browser global is typed through WebIDL (lib/web.webidl, from w3c/webref) as `use c`
//! is through the C headers: a member the global's interface lacks, an attribute called, or a call with an argument
//! count no overload takes is a compile-time error, natively (node) and in the page alike
use crate::common::fails_with;
use crate::is;

#[test]
fn a_member_the_interface_lacks_is_an_error_with_its_near_miss() {
	fails_with("use js crypto; crypto.randomUuid()", "did you mean randomUUID");
	fails_with("use js localStorage; localStorage.set(\"k\", \"v\")", "Storage");
}

#[test]
fn a_call_with_a_count_no_overload_takes_is_an_error() {
	fails_with("use js localStorage; localStorage.setItem(\"k\")", "setItem(DOMString key, DOMString value)");
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
