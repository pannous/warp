//! The standard library module net (notes/stdlib.md section 7): post sends a text and gives the answer's text, ureq
//! natively and a synchronous request in the browser; the echo server (common::echo_url: a warp server natively, the
//! test page's server in the browser) answers what it got, as httpbin.org does
use crate::common::echo_url;
use crate::is_online;

#[test]
fn use_net_posts_a_text() {
	is_online!(&format!("use net; use json; answer = parse_json(post(\"{}\", \"hello warp\")); answer.data", echo_url()), "hello warp");
}

#[test]
fn post_sends_the_headers_of_a_map() {
	is_online!(&format!("use net; use json; answer = parse_json(post(\"{}\", \"hi\", {{\"X-Warp\": \"yes\"}})); answer.headers[\"X-Warp\"]", echo_url()), "yes");
}
