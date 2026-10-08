//! The standard library module net (notes/stdlib.md section 7): post sends a text and gives the answer's text, ureq
//! natively and a synchronous request in the browser; httpbin.org echoes what it got
use crate::is;

#[test]
fn use_net_posts_a_text() {
	is!("use net; use json; answer = parse_json(post(\"https://httpbin.org/post\", \"hello warp\")); answer.data", "hello warp");
}
