//! card js-callbacks: JavaScript's asynchronous and higher-order members. A promise is awaited natively (node); in the
//! page a foreign call is synchronous, so a promise is a clear error instead of an opaque object. A warp function given
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
fn a_promise_in_the_page_is_a_clear_error() {
	fails_with("use js Promise; Promise.resolve(1)", "gives a promise");
	fails_with("use js Promise; Promise.reject(1)", "gives a promise");
}

#[test]
fn a_warp_function_is_called_back_by_javascript() {
	is!("use js Array; use js JSON; JSON.stringify(Array.from([1, 2, 3], x => x * 2))", "[2,4,6]");
	is!("use js Array; use js JSON; k = 10; JSON.stringify(Array.from([1, 2], x => x + k))", "[11,12]");
	is!("use js Array; use js JSON; JSON.stringify(Array.from([5, 6], (x, i) => i))", "[0,1]");
}

// natively node waits for the callback's answer on its stdin: a request there is not read yet (card js-nested-callbacks)
#[test]
fn a_warp_function_called_back_calls_javascript_in_the_page() {
	let code = "use js Array; use js Math; use js JSON; JSON.stringify(Array.from([1, 3], x => Math.max(x, 2)))";
	#[cfg(not(feature = "native"))]
	is!(code, "[2,3]");
	#[cfg(feature = "native")]
	fails_with(code, "cannot call foreign code itself yet");
}
