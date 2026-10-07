//! `serve 8080 { get "/api/users" { users } post "/echo" { request.body } }` (card web-server, notes/web_framework.md):
//! lowering/web_server.rs makes each route a function `route·N(request:any)` and the statement the host call
//! `serve_routes(port, [[method, path, function] …])`; this serves HTTP on the port and calls the route's function with
//! the request {method, path, query, body} (a JSON body parsed). The value answers: a text as text/plain, any other
//! value as JSON (as std json's to_json writes it); no route is 404, a failing route 500 with its message.

use crate::node::Node;
use std::cell::Cell;

const JSON_TYPE: &str = "application/json";
const TEXT_TYPE: &str = "text/plain; charset=utf-8";
const NOT_FOUND: u16 = 404;
const FAILED: u16 = 500;

thread_local! {
	/// How many requests this thread's next serve answers before it returns: 0 serves on (tests stop it so)
	static REQUEST_LIMIT: Cell<usize> = const { Cell::new(0) };
}

/// The next serve on this thread returns after `requests` requests (a test's server ends)
pub fn stop_after(requests: usize) {
	REQUEST_LIMIT.with(|limit| limit.set(requests));
}

/// A route of the program: method, path and the function answering it
pub struct Route {
	pub method: String,
	pub path: String,
	pub function: String,
}

/// What a route's function gave, as the HTTP answer: status, content type, body
pub struct Answer {
	pub status: u16,
	pub content_type: &'static str,
	pub body: String,
}

impl Answer {
	pub fn of(value: &Node) -> Answer {
		match value.drop_meta() {
			Node::Text(text) => Answer { status: 200, content_type: TEXT_TYPE, body: text.clone() },
			Node::Char(character) => Answer { status: 200, content_type: TEXT_TYPE, body: character.to_string() },
			Node::Error(message) => Answer::failed(&message.to_string()),
			other => Answer { status: 200, content_type: JSON_TYPE, body: crate::foreign::json_of(other).to_string() },
		}
	}

	pub fn failed(message: &str) -> Answer {
		Answer { status: FAILED, content_type: TEXT_TYPE, body: message.to_string() }
	}
}

/// The routes of `[[method, path, function] …]`
pub fn routes_of(routes: &Node) -> Vec<Route> {
	let Node::List(items, _, _) = routes.drop_meta() else { return vec![] };
	items.iter().filter_map(|route| match route.drop_meta() {
		Node::List(parts, _, _) if parts.len() == 3 => Some(Route { method: text_of(&parts[0]), path: text_of(&parts[1]), function: text_of(&parts[2]) }),
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

/// Serve on `port` until the request limit (if any): `answer(route, request)` runs the route's function
pub fn serve(port: u16, routes: &[Route], mut answer: impl FnMut(&Route, Node) -> Answer) -> Result<(), String> {
	let server = tiny_http::Server::http(("0.0.0.0", port)).map_err(|problem| format!("serve {port}: {problem}"))?;
	let limit = REQUEST_LIMIT.with(|limit| limit.replace(0));
	let mut served = 0;
	for mut request in server.incoming_requests() {
		let method = request.method().as_str().to_uppercase();
		let (path, query) = request.url().split_once('?').map_or((request.url().to_string(), String::new()), |(path, query)| (path.to_string(), query.to_string()));
		let mut body = String::new();
		let _ = request.as_reader().read_to_string(&mut body);
		let answered = match routes.iter().find(|route| route.method == method && route.path == path) {
			Some(route) => answer(route, request_node(&method, &path, &query, body)),
			None => Answer { status: NOT_FOUND, content_type: TEXT_TYPE, body: format!("no route {method} {path}") },
		};
		let header = tiny_http::Header::from_bytes("Content-Type", answered.content_type).expect("a valid header");
		let _ = request.respond(tiny_http::Response::from_string(answered.body).with_status_code(answered.status).with_header(header));
		served += 1;
		if limit > 0 && served >= limit {
			break;
		}
	}
	Ok(())
}

/// The request as the route's `request`: {method, path, query, body}, a JSON body parsed into its value
fn request_node(method: &str, path: &str, query: &str, body: String) -> Node {
	let body = serde_json::from_str::<serde_json::Value>(&body).ok().filter(|value| value.is_object() || value.is_array())
		.map_or(Node::Text(body), |value| crate::foreign::node_of(&value));
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
