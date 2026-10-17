//! card js-callbacks: JavaScript's asynchronous and higher-order members. A promise is awaited natively (node) and in a
//! page with JSPI (card jspi-page); in one without (Safari) it is a clear error instead of an opaque object. A warp function given
//! to JavaScript is a JavaScript function calling it back, with as many of its arguments as it takes
use crate::common::fails_with;
use crate::is;

#[cfg(feature = "native")]
#[test]
fn a_promise_is_awaited_natively() {
	is!("use js Promise; Promise.resolve(1)", 1);
}

#[cfg(not(feature = "native"))]
#[test]
fn a_promise_in_the_page_is_awaited_with_jspi_else_a_clear_error() {
	if crate::common::jspi() {
		is!("use js Promise; Promise.resolve(1)", 1);
		fails_with("use js Promise; Promise.reject(1)", "rejected");
	} else {
		fails_with("use js Promise; Promise.resolve(1)", "gives a promise");
		fails_with("use js Promise; Promise.reject(1)", "gives a promise");
	}
}

#[test]
fn a_warp_function_is_called_back_by_javascript() {
	is!("use js Array; use js JSON; JSON.stringify(Array.from([1, 2, 3], x => x * 2))", "[2,4,6]");
	is!("use js Array; use js JSON; k = 10; JSON.stringify(Array.from([1, 2], x => x + k))", "[11,12]");
	is!("use js Array; use js JSON; JSON.stringify(Array.from([5, 6], (x, i) => i))", "[0,1]");
}

// natively node answers the warp function's own foreign calls while it waits for its answer (card js-nested); a promise
// there cannot be waited for
#[test]
fn a_warp_function_called_back_calls_javascript() {
	is!("use js Array; use js Math; use js JSON; JSON.stringify(Array.from([1, 3], x => Math.max(x, 2)))", "[2,3]");
	is!("use js Array; use js JSON; JSON.stringify(Array.from([[1], [2, 3]], xs => JSON.stringify(Array.from(xs, x => x * 10))))", "[\"[10]\",\"[20,30]\"]");
	#[cfg(feature = "native")]
	fails_with("use js Array; use js Promise; Array.from([1], x => Promise.resolve(x))", "cannot wait for");
}

// a JavaScript frame between main and the call: the page cannot suspend it (card jspi-page), as natively node cannot
#[cfg(not(feature = "native"))]
#[test]
fn a_promise_in_a_warp_function_called_back_is_a_clear_error_in_the_page() {
	fails_with("use js Array; use js Promise; Array.from([1], x => Promise.resolve(x))", "cannot wait for");
}
