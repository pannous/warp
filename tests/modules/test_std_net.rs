//! The standard library module net (notes/stdlib.md section 7): post sends a text and gives the answer's text, ureq
//! natively and a synchronous request in the browser; the echo server (common::echo_url: a warp server natively, the
//! test page's server in the browser) answers what it got, as httpbin.org does
use crate::common::echo_url;
use crate::is_online;

#[test]
fn use_net_posts_a_text() {
	is_online!(
		&format!("use net; use json; answer = parse_json(post(\"{}\", \"hello warp\")); answer.data", echo_url()),
		"hello warp"
	);
}

#[test]
fn post_sends_the_headers_of_a_map() {
	is_online!(&format!("use net; use json; answer = parse_json(post(\"{}\", \"hi\", {{\"X-Warp\": \"yes\"}})); answer.headers[\"X-Warp\"]", echo_url()), "yes");
}

// common::online_eval tries once more only when no answer came (connection refused here), never on an answer
#[test]
#[cfg(feature = "native")]
fn only_a_network_failure_is_retried() {
	use crate::common::{network_failure_of, retried_on_network_failure, serve};
	let attempts_at = |url: String| {
		let attempts = std::cell::Cell::new(0);
		let code = format!("use net; post(\"{url}\", \"x\")");
		retried_on_network_failure(
			&code,
			|| {
				attempts.set(attempts.get() + 1);
				warp::wasm_emitter::eval(&code)
			},
			network_failure_of,
		);
		attempts.get()
	};
	let closed_port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port(); // listener dropped: refused
	assert_eq!(attempts_at(format!("http://127.0.0.1:{closed_port}/post")), 2);
	assert_eq!(attempts_at(serve("401 Unauthorized", "no")), 1);
}
