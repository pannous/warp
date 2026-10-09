//! `serve 8080 { get "/api/users" { users } post "/echo" { request.body } }` (card web-server, notes/web_framework.md):
//! lowering/web_server.rs makes each route a function `route·N(request:any)` and the statement the host call
//! `serve_routes(port, [[method, path, function] …])`; this serves HTTP on the port and calls the route's function with
//! the request {method, path, query, body} (a JSON body parsed). The value answers: a text as text/plain, any other
//! value as JSON (as std json's to_json writes it); no route is 404, a failing route 500 with its message.

use crate::node::Node;
use crate::site::ServedSite;
use std::cell::Cell;

const JSON_TYPE: &str = "application/json";
const TEXT_TYPE: &str = "text/plain; charset=utf-8";
/// A reply of this type is a text (host-tasks.js TEXT_REPLY_TYPE)
pub const TEXT_REPLY_TYPE: &str = "text/plain";
const NOT_FOUND: u16 = 404;
const FAILED: u16 = 500;
const SITE_METHOD: &str = "GET";

thread_local! {
	/// How many requests this thread's next serve answers before it returns: 0 serves on (tests stop it so)
	static REQUEST_LIMIT: Cell<usize> = const { Cell::new(0) };
}

/// The next serve on this thread returns after `requests` requests (a test's server ends)
pub fn stop_after(requests: usize) {
	REQUEST_LIMIT.with(|limit| limit.set(requests));
}

/// The request limit stop_after set for this serve, 0 serving on; the next serve starts unlimited
pub(crate) fn take_request_limit() -> usize {
	REQUEST_LIMIT.with(|limit| limit.replace(0))
}

/// A route of the program: method, path, the function answering it and whether its value is a list
pub struct Route {
	pub method: String,
	pub path: String,
	pub function: String,
	pub lists: bool,
}

impl Route {
	/// The answer of the route's value; ø is the empty list, which reads back as nothing, so a list route says []
	pub fn answer_of(&self, value: &Node) -> Answer {
		match value.drop_meta() {
			Node::Empty if self.lists => Answer { status: 200, content_type: JSON_TYPE, body: b"[]".to_vec() },
			_ => Answer::of(value),
		}
	}
}

/// What a route's function gave, as the HTTP answer: status, content type, body
pub struct Answer {
	pub status: u16,
	pub content_type: &'static str,
	pub body: Vec<u8>,
}

impl Answer {
	pub fn of(value: &Node) -> Answer {
		match value.drop_meta() {
			Node::Text(text) => Answer { status: 200, content_type: TEXT_TYPE, body: text.clone().into_bytes() },
			Node::Char(character) => Answer { status: 200, content_type: TEXT_TYPE, body: character.to_string().into_bytes() },
			Node::Error(message) => Answer::failed(&message.to_string()),
			other => Answer { status: 200, content_type: JSON_TYPE, body: crate::foreign::json_of(other).to_string().into_bytes() },
		}
	}

	pub fn failed(message: &str) -> Answer {
		Answer { status: FAILED, content_type: TEXT_TYPE, body: message.as_bytes().to_vec() }
	}
}

/// The routes of `[[method, path, function] …]`, a list route marked `[method, path, function, serve::LIST_ANSWER]`
pub fn routes_of(routes: &Node) -> Vec<Route> {
	let Node::List(items, _, _) = routes.drop_meta() else { return vec![] };
	items.iter().filter_map(|route| match route.drop_meta() {
		Node::List(parts, _, _) if (3..=4).contains(&parts.len()) => Some(Route {
			method: text_of(&parts[0]),
			path: text_of(&parts[1]),
			function: text_of(&parts[2]),
			lists: parts.get(3).is_some_and(|mark| text_of(mark) == crate::serve::LIST_ANSWER),
		}),
		_ => None,
	}).collect()
}

/// A text of the route table; the path "/" arrives as a one-character Char
fn text_of(node: &Node) -> String {
	match node.drop_meta() {
		Node::Char(character) => character.to_string(),
		other => other.name(),
	}
}

/// Serve on `port` until the request limit (if any): `answer(route, request)` runs the route's function; a GET no route
/// takes is a file of the program's `site` (src/site.rs), its page at /
pub fn serve(port: u16, routes: &[Route], site: &ServedSite, mut answer: impl FnMut(&Route, Node) -> Answer) -> Result<(), String> {
	let server = tiny_http::Server::http(("0.0.0.0", port)).map_err(|problem| format!("serve {port}: {problem}"))?;
	let limit = take_request_limit();
	let mut served = 0;
	for mut request in server.incoming_requests() {
		let method = request.method().as_str().to_uppercase();
		let (path, query) = request.url().split_once('?').map_or((request.url().to_string(), String::new()), |(path, query)| (path.to_string(), query.to_string()));
		let path = percent_decoded(&path);
		let mut body = String::new();
		let _ = request.as_reader().read_to_string(&mut body);
		let answered = match routes.iter().find(|route| route.method == method && route.path == path) {
			Some(route) => answer(route, request_node(&method, &path, &query, body)),
			None => site_file(site, &method, &path).unwrap_or_else(|| Answer { status: NOT_FOUND, content_type: TEXT_TYPE, body: format!("no route {method} {path}").into_bytes() }),
		};
		let header = tiny_http::Header::from_bytes("Content-Type", answered.content_type).expect("a valid header");
		let _ = request.respond(tiny_http::Response::from_data(answered.body).with_status_code(answered.status).with_header(header));
		served += 1;
		if limit > 0 && served >= limit {
			break;
		}
	}
	Ok(())
}

/// `GET /` is the site's page, `GET /app.wasm` its module and so on
fn site_file(site: &ServedSite, method: &str, path: &str) -> Option<Answer> {
	if method != SITE_METHOD {
		return None;
	}
	Some(match site.file_at(path)? {
		Ok((name, body)) => Answer { status: 200, content_type: crate::site::content_type(&name), body },
		Err(failure) => Answer::failed(&failure),
	})
}

/// An HTTP body as a value: a JSON object or array parsed into its map or list, any other body the text
pub fn value_of_body(body: String) -> Node {
	serde_json::from_str::<serde_json::Value>(&body).ok().filter(|value| value.is_object() || value.is_array())
		.map_or(Node::Text(body), |value| crate::foreign::node_of(&value))
}

/// `/rpc/route%C2%B7data%C2%B70` → `/rpc/route·data·0` (a browser's fetch encodes the path); an invalid escape stays
fn percent_decoded(path: &str) -> String {
	let bytes = path.as_bytes();
	let mut decoded = Vec::with_capacity(bytes.len());
	let mut index = 0;
	while index < bytes.len() {
		let escaped = (bytes[index] == b'%').then(|| path.get(index + 1..index + 3)).flatten().and_then(|hex| u8::from_str_radix(hex, 16).ok());
		match escaped {
			Some(byte) => { decoded.push(byte); index += 3; }
			None => { decoded.push(bytes[index]); index += 1; }
		}
	}
	String::from_utf8(decoded).unwrap_or_else(|_| path.to_string())
}

/// The request as the route's `request`: {method, path, query, body}, a JSON body parsed into its value
fn request_node(method: &str, path: &str, query: &str, body: String) -> Node {
	let body = value_of_body(body);
	let query = query.split('&').filter(|pair| !pair.is_empty()).map(|pair| {
		let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
		entry(key, Node::Text(value.replace('+', " ")))
	}).collect();
	Node::List(vec![
		entry("method", Node::Text(method.to_string())),
		entry("path", Node::Text(path.to_string())),
		entry("query", Node::List(query, crate::node::Bracket::Curly, crate::node::Separator::Space)),
		entry("body", body),
	], crate::node::Bracket::Curly, crate::node::Separator::Space)
}

fn entry(key: &str, value: Node) -> Node {
	Node::Key(Box::new(Node::Symbol(key.to_string())), crate::operators::Op::Colon, Box::new(value))
}
