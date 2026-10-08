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

// a nullable result is the optional type: getItem's DOMString? is text? (ø or a text, optional_casts.rs)
#[test]
fn a_nullable_result_is_optional() {
	eq!(warp::web_idl::optional_result_type("Storage", "getItem", true), Some("text?".to_string()));
	eq!(warp::web_idl::optional_result_type("Crypto", "randomUUID", true), Some("text".to_string()));
	eq!(warp::web_idl::optional_result_type("Navigator", "clipboard", false), None);
}

// the DOM: document is a Document; what an operation gives is typed too (getElementById: Element), and an Element
// takes the members of the elements deriving from it (an input's value)
#[cfg(feature = "native")] // a Worker has no document
#[test]
fn the_dom_is_typed() {
	fails_with("use js document; document.getElementByID(\"x\")", "did you mean getElementById");
	fails_with("use js document; document.getElementById(\"x\").innerHtml", "document.getElementById(…) (Element in WebIDL) has no member innerHtml; did you mean innerHTML");
	fails_with("use js document; e = document.querySelector(\"p\"); e.classList.ad(\"x\")", "DOMTokenList");
	fails_with("use js document; document.body.appendChild(1, 2)", "appendChild(Node node)");
	fails_with("use js document; document.getElementById(\"x\").valu", "did you mean value");
	assert!(warp::web_idl::check_member("Element", "e", "value", None).is_ok()); // HTMLInputElement's
	eq!(warp::web_idl::result_interface("Document", "getElementById", true), Some("Element".to_string()));
	eq!(warp::web_idl::result_interface("Document", "body", false), Some("HTMLElement".to_string()));
}

// the global object (self, globalThis; window on a page) has WindowOrWorkerGlobalScope's members typed: fetch gives a
// Response (its promise awaited), atob a text; members it is not bundled with stay unchecked
#[test]
fn the_global_object_is_typed() {
	is!("use js self; self.atob(\"aGk=\").upper()", "HI");
	is!("use js globalThis; count globalThis.btoa(\"hi\")", 4);
	fails_with("use js self; r = self.fetch(\"https://example.com\"); r.stauts", "(Response in WebIDL) has no member stauts; did you mean status");
	eq!(warp::web_idl::result_interface("Window", "fetch", true), Some("Response".to_string()));
	eq!(warp::web_idl::result_type("Response", "text", true), Some("text")); // Promise<USVString>
	eq!(warp::web_idl::result_type("Response", "status", false), Some("int"));
}

#[cfg(not(feature = "native"))]
#[test]
fn a_worker_has_no_window() {
	fails_with("use js window; window.atob(\"aGk=\")", "self is the Worker's global");
}

// a constructor is called as a function (JavaScript's `new`): checked against WebIDL's constructors, the value typed
#[test]
fn a_constructor_is_a_plain_call() {
	is!("use js URL; u = URL(\"https://a.b/c?d=1\"); u.pathname", "/c");
	is!("use js URLSearchParams; p = URLSearchParams(\"a=1&b=2\"); p.get(\"b\")", "2");
	is!("use js Blob; b = Blob([\"abc\"]); b.size", 3);
	is!("use js Date; d = Date(0); d.getTime()", 0); // ECMAScript: constructed, unchecked
	fails_with("use js URL; URL(\"https://a.b/c\").pathnam", "did you mean pathname");
	fails_with("use js URL; URL()", "URL(USVString url, optional USVString base), not 0 arguments");
	fails_with("use js Location; Location()", "Location has no constructor in WebIDL");
}

// canvas 2D and WebGPU's entry points: getContext of a literal id gives that context's interface
#[cfg(feature = "native")] // a page's canvas; node has none, so only the checks run
#[test]
fn a_canvas_context_is_typed() {
	fails_with("use js document; c = document.getElementById(\"c\"); ctx = c.getContext(\"2d\"); ctx.fillRec(0, 0, 1, 1)", "(CanvasRenderingContext2D in WebIDL) has no member fillRec; did you mean fillRect");
	fails_with("use js document; ctx = document.getElementById(\"c\").getContext(\"2d\"); ctx.fillRect(0, 0, 1)", "not 3 arguments");
	fails_with("use js document; ctx = document.getElementById(\"c\").getContext(\"webgpu\"); ctx.configur(1)", "did you mean configure");
	fails_with("use js navigator; navigator.gpu.requestAdaptor()", "(GPU in WebIDL) has no member requestAdaptor; did you mean requestAdapter");
	eq!(warp::web_idl::context_interface("OffscreenCanvas", "getContext", "2d"), Some("OffscreenCanvasRenderingContext2D".to_string()));
}

// a Worker draws on an OffscreenCanvas
#[cfg(not(feature = "native"))]
#[test]
fn an_offscreen_canvas_draws_in_a_worker() {
	is!("use js OffscreenCanvas; c = OffscreenCanvas(4, 4); ctx = c.getContext(\"2d\"); ctx.fillRect(0, 0, 2, 2); c.width", 4);
	fails_with("use js OffscreenCanvas; c = OffscreenCanvas(4, 4); ctx = c.getContext(\"2d\"); ctx.fillRec(0, 0, 2, 2)", "did you mean fillRect");
}
