//! `serve 8080 { get "/api/users" { users } post "/echo" { request.body } }` (card web-server, notes/web_framework.md):
//! lowering/web_server.rs makes each route a function `route·N(request:any)` and the statement the host call
//! `serve_routes(port, [[method, path, function] …])`; this serves HTTP on the port and calls the route's function with
//! the request {method, path, query, body} (a JSON body parsed). The value answers: a text as text/plain, any other
//! value as JSON (as std json's to_json writes it, an instance as a plain object); a route's path may have typed
//! parameters as a page route's (`get "/api/users/:id:int"`). No route is 404, a failing route 500 with its message,
//! a refused request 400 or 404 (Route::failure_answer); a page path no route of the page matches is 404 too.

use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::site::ServedSite;
use std::cell::Cell;

const JSON_TYPE: &str = "application/json";
/// The address a server listens on: every interface, or WARP_SERVE_ADDRESS (warp-lambda: 127.0.0.2, reachable only
/// through its proxy, web/hosting/server)
const SERVE_ADDRESS_VARIABLE: &str = "WARP_SERVE_ADDRESS";
const EVERY_INTERFACE: &str = "0.0.0.0";
const TEXT_TYPE: &str = "text/plain; charset=utf-8";
/// A reply of this type is a text (host-tasks.js TEXT_REPLY_TYPE)
pub const TEXT_REPLY_TYPE: &str = "text/plain";
const BAD_REQUEST: u16 = 400;
const NOT_FOUND: u16 = 404;
const FAILED: u16 = 500;
const MISSING_FIELD: &str = "missing field ";
/// The request's parts that hold its fields (request_node)
const FIELD_PARTS: [&str; 2] = ["body", "query"];
const PAGE_BODY_TAG: &str = "<body>";
/// Routes under these paths answer a failure as JSON `{"error": …}`, the others as text
const JSON_PATHS: [&str; 2] = ["/api/", "/rpc/"];
/// The runtime errors of a lookup that found nothing (`users#id` of a missing row): the request names no such thing, 404
const NOT_FOUND_ERRORS: [&str; 2] = ["index_out_of_range", "key_not_found"];
const SITE_METHOD: &str = "GET";
/// A body of this type is the fields of an HTML form, `title=buy+milk&done=on`
const FORM_TYPE: &str = "application/x-www-form-urlencoded";
/// A browser's form asks for a page; it gets See Other back to the page it was sent from (post/redirect/get)
const PAGE_TYPE: &str = "text/html";
const SEE_OTHER: u16 = 303;
const HOME: &str = "/";
const PATH_SEPARATOR: char = '/';

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

/// Why a route failed (host.rs route_failure): its message, and whether the program raised it (`raise "…"`, a refusal
/// such as a form's validation) rather than broke
pub struct RouteFailure {
	pub message: String,
	pub raised: bool,
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
			Node::Error(message) => self.failed(&message.to_string()),
			_ => Answer::of(value),
		}
	}

	/// The answer of the route failing with `message`: JSON `{"error": message}` where a program reads JSON (an
	/// /api/ or /rpc/ path), else the message as text. A lookup missing is 404 only where the path named what to look up
	/// (`/api/users/:id`); in a route without parameters (`get "/boom" { xs#5 }`) it is the server's fault, 500
	pub fn failed(&self, message: &str) -> Answer {
		self.failed_with(self.lookup_status(message), message)
	}

	fn failed_with(&self, status: u16, message: &str) -> Answer {
		answer_failing(&self.path, status, message)
	}

	fn lookup_status(&self, message: &str) -> u16 {
		let found_nothing = message.ends_with(crate::wasm_emitter::EMPTY_RANGE) || failure_status(message) == NOT_FOUND;
		if found_nothing && self.path.contains(crate::routes::PARAMETER_MARK) { NOT_FOUND } else { FAILED }
	}

	/// The answer of a failure by its cause: a field the request lacks is 400 "missing field …", a path parameter's
	/// lookup that found nothing (`(todos where it.id == id)#1`, `users#id`) 404, anything else a bug of the program, 500
	fn failure_answer(&self, failure: &RouteFailure, request: &Node) -> Answer {
		let missing = failure.message.strip_prefix(crate::wasm_emitter::list_ops::NO_FIELD_MESSAGE).filter(|field| !has_field(request, field));
		match missing {
			Some(field) => self.failed_with(BAD_REQUEST, &format!("{MISSING_FIELD}{field}")),
			None => self.failed(&failure.message),
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
			other => Answer { status: 200, content_type: JSON_TYPE, body: crate::foreign::json_of(&without_classes(other)).to_string().into_bytes() },
		}
	}

	pub fn failed(message: &str) -> Answer {
		answer_failing("", failure_status(message), message)
	}

	/// A page with `message` shown at its top, as an alert: the first thing after <body>, else before the whole page
	fn with_message(self, message: &str) -> Answer {
		let page = String::from_utf8_lossy(&self.body).to_string();
		let alert = format!("<p role=\"alert\">{}</p>", crate::site::escaped(message));
		let at = page.find(PAGE_BODY_TAG).map_or(0, |start| start + PAGE_BODY_TAG.len());
		Answer { body: format!("{}{alert}{}", &page[..at], &page[at..]).into_bytes(), ..self }
	}
}

/// A reply's instances as plain objects, `User{name:"Ann"}` as `{"name":"Ann"}` (supervisor default 2026-10-09, undoable:
/// no class wrapper in /api and /rpc JSON); an object's own entries stay, so a map field `address:{…}` keeps its name
fn without_classes(value: &Node) -> Node {
	match value.drop_meta() {
		Node::Key(_, Op::None, body) if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => without_classes(body),
		Node::List(items, Bracket::Curly, separator) => Node::List(items.iter().map(|item| match item.drop_meta() {
			Node::Key(key, op, entry) => Node::Key(key.clone(), *op, Box::new(without_classes(entry))),
			other => without_classes(other),
		}).collect(), Bracket::Curly, separator.clone()),
		Node::List(items, bracket, separator) => Node::List(items.iter().map(without_classes).collect(), bracket.clone(), separator.clone()),
		other => other.clone(),
	}
}

/// The answer `status` with `message`: JSON `{"error": message}` where a program reads JSON (an /api/ or /rpc/ path),
/// else the message as text
fn answer_failing(path: &str, status: u16, message: &str) -> Answer {
	match is_json_path(path) {
		true => Answer { status, content_type: JSON_TYPE, body: serde_json::json!({ "error": message }).to_string().into_bytes() },
		false => Answer { status, content_type: TEXT_TYPE, body: message.as_bytes().to_vec() },
	}
}

/// An /api/ or /rpc/ path, which a program reads: its answers are JSON, and no page stands there
fn is_json_path(path: &str) -> bool {
	JSON_PATHS.iter().any(|prefix| path.starts_with(prefix))
}

/// 404 for a lookup that found nothing, else 500
fn failure_status(message: &str) -> u16 {
	match NOT_FOUND_ERRORS.iter().any(|error| message.contains(&error.replace('_', " "))) {
		true => NOT_FOUND,
		false => FAILED,
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
	crate::routes::pattern_text(node)
}

/// Serve on `port` until the request limit (if any): `answer(route, request)` runs the route's function; a GET no route
/// takes is a file of the program's `site` (src/site.rs), its page at /
pub fn serve(port: u16, routes: &[Route], site: &ServedSite, mut answer: impl FnMut(&Route, Node) -> Result<Answer, RouteFailure>) -> Result<(), String> {
	let address = std::env::var(SERVE_ADDRESS_VARIABLE).unwrap_or_else(|_| EVERY_INTERFACE.to_string());
	let server = tiny_http::Server::http((address.as_str(), port)).map_err(|problem| format!("serve {address}:{port}: {problem}"))?;
	let limit = take_request_limit();
	let mut served = 0;
	for mut request in server.incoming_requests() {
		let method = request.method().as_str().to_uppercase();
		let (path, query) = request.url().split_once('?').map_or((request.url().to_string(), String::new()), |(path, query)| (path.to_string(), query.to_string()));
		let path = percent_decoded(&path);
		let mut body = String::new();
		let _ = request.as_reader().read_to_string(&mut body);
		let header_of = |name: &'static str| request.headers().iter().find(|header| header.field.equiv(name)).map(|header| header.value.to_string());
		let is_form = header_of("Content-Type").is_some_and(|content_type| content_type.starts_with(FORM_TYPE));
		let from_a_browser_form = is_form && header_of("Accept").is_some_and(|accepted| accepted.contains(PAGE_TYPE));
		let back = header_of("Referer").map_or(HOME.to_string(), |referer| page_of(&referer));
		let body = if is_form { form_fields(&body) } else { value_of_body(body) };
		let mut answer_at = |method: &str, path: &str, query: &str, body: Node| match route_at(routes, method, path) {
			Some(route) => {
				let request = request_node(method, path, query, body);
				answer(route, request.clone()).map_err(|failure| (route, failure, request))
			}
			None => Ok(site_file(site, method, path).unwrap_or_else(|| answer_failing(path, NOT_FOUND, &format!("no route {method} {path}")))),
		};
		let answered = match answer_at(&method, &path, &query, body) {
			Ok(answered) => answered,
			// a refusal the program raised to a browser's form: the page it came from again, saying why
			Err((_, failure, _)) if failure.raised && from_a_browser_form => {
				let (page_path, page_query) = back.split_once('?').unwrap_or((&back, ""));
				match answer_at(SITE_METHOD, page_path, page_query, Node::Empty) {
					Ok(page) if page.status == 200 => Answer { status: BAD_REQUEST, ..page.with_message(&failure.message) },
					_ => Answer { status: BAD_REQUEST, ..Answer::failed(&failure.message) },
				}
			}
			Err((route, failure, request)) => route.failure_answer(&failure, &request),
		};
		let content_type = tiny_http::Header::from_bytes("Content-Type", answered.content_type).expect("a valid header");
		let response = match from_a_browser_form && answered.status == 200 {
			true => tiny_http::Response::from_data(vec![]).with_status_code(SEE_OTHER).with_header(tiny_http::Header::from_bytes("Location", back).expect("a valid header")),
			false => tiny_http::Response::from_data(answered.body).with_status_code(answered.status).with_header(content_type),
		};
		let _ = request.respond(response);
		served += 1;
		if limit > 0 && served >= limit {
			break;
		}
	}
	Ok(())
}

/// The route answering `method path`: the one of exactly that path, else the first whose pattern matches it
/// (`get "/api/users/:id:int"`)
fn route_at<'a>(routes: &'a [Route], method: &str, path: &str) -> Option<&'a Route> {
	let of_method = || routes.iter().filter(|route| route.method == method);
	of_method().find(|route| route.path == path).or_else(|| of_method().find(|route| crate::routes::path_matches(&route.path, path)))
}

/// `GET /` is the site's page, `GET /app.wasm` its module and so on; the page at a path none of its routes matches is
/// not found
fn site_file(site: &ServedSite, method: &str, path: &str) -> Option<Answer> {
	if method != SITE_METHOD || is_json_path(path) {
		return None;
	}
	let status = if site.matches_no_route_at(path) { NOT_FOUND } else { 200 };
	Some(match site.file_at(path)? {
		Ok((name, body)) => Answer { status, content_type: crate::site::content_type(&name), body },
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

/// The path and query of a Referer URL, where a browser's form goes back to
fn page_of(referer: &str) -> String {
	let after_scheme = referer.split_once("://").map_or(referer, |(_, rest)| rest);
	after_scheme.find(PATH_SEPARATOR).map_or(HOME.to_string(), |start| after_scheme[start..].to_string())
}

/// `title=buy+milk&done=on`, a form's body or a query, as the map {title: "buy milk", done: "on"}
fn form_fields(encoded: &str) -> Node {
	let fields = encoded.split('&').filter(|pair| !pair.is_empty()).map(|pair| {
		let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
		entry(&percent_decoded(&key.replace('+', " ")), Node::Text(percent_decoded(&value.replace('+', " "))))
	}).collect();
	Node::List(fields, crate::node::Bracket::Curly, crate::node::Separator::Space)
}

/// The request as the route's `request`: {method, path, query, body}, the body a JSON value or a form's fields
fn request_node(method: &str, path: &str, query: &str, body: Node) -> Node {
	Node::List(vec![
		entry("method", Node::Text(method.to_string())),
		entry("path", Node::Text(path.to_string())),
		entry("query", form_fields(query)),
		entry("body", body),
	], crate::node::Bracket::Curly, crate::node::Separator::Space)
}

/// Whether the request's body or query holds the field
fn has_field(request: &Node, field: &str) -> bool {
	FIELD_PARTS.iter().filter_map(|part| entry_value(request, part)).any(|fields| entry_value(fields, field).is_some())
}

/// The value of a map's entry by key
fn entry_value<'map>(map: &'map Node, key: &str) -> Option<&'map Node> {
	let Node::List(entries, _, _) = map.drop_meta() else { return None };
	entries.iter().find_map(|entry| match entry.drop_meta() {
		Node::Key(name, _, value) if name.name() == key => Some(value.as_ref()),
		_ => None,
	})
}

fn entry(key: &str, value: Node) -> Node {
	Node::Key(Box::new(Node::Symbol(key.to_string())), crate::operators::Op::Colon, Box::new(value))
}
