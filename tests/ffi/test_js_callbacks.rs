//! card js-callbacks: JavaScript's asynchronous and higher-order members. A promise is awaited natively (node); in the
//! page a foreign call is synchronous, so a promise is a clear error instead of an opaque object
#[cfg(not(feature = "native"))]
use crate::common::fails_with;
#[cfg(feature = "native")]
use crate::is;

#[cfg(feature = "native")]
#[test]
fn a_promise_is_awaited_natively() {
	is!("use js Promise; Promise.resolve(1)", 1);
}

#[cfg(not(feature = "native"))]
#[test]
fn a_promise_in_the_page_is_a_clear_error() {
	fails_with("use js Promise; Promise.resolve(1)", "gives a promise");
	fails_with("use js Promise; Promise.reject(1)", "gives a promise");
}
