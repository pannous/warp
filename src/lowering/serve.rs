//! `serve 8080 { get "/api/users" { users } post "/echo" { request.body } }` (card web-server): each route becomes the
//! function `route·N(request:any) := body` and the statement the host call `serve_routes(8080, [["GET",
//! "/api/users", "route·0"] …])`, which serves until it is stopped (src/web_server.rs natively; the playground says
//! loudly that it cannot serve).

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const SERVE_WORD: &str = "serve";
const ROUTE_PREFIX: &str = "route·";
const REQUEST_WORD: &str = "request";
const ANY_TYPE: &str = "any";
const METHODS: [&str; 5] = ["get", "post", "put", "delete", "patch"];

/// A route as written: its method (upper case), path and body
type Route = (String, Node, Node);

pub fn lower(program: Node) -> Node {
	match program {
		Node::List(statements, bracket, separator) if statements.iter().any(|statement| served(statement).is_some()) => {
			let mut routes = 0;
			let statements = statements.into_iter().flat_map(|statement| match served(&statement) {
				Some((port, routed)) => serving(port, routed, &mut routes),
				None => vec![statement],
			}).collect();
			Node::List(statements, bracket, separator)
		}
		single if served(&single).is_some() => {
			let (port, routed) = served(&single).expect("checked");
			Node::List(serving(port, routed, &mut 0), Bracket::None, Separator::Semicolon)
		}
		other => other,
	}
}

/// `serve port {routes}`: the port and each route's method, path and body
fn served(statement: &Node) -> Option<(Node, Vec<Route>)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [serve, port, block] = items.as_slice() else { return None };
	let Node::List(routes, Bracket::Curly, _) = block.drop_meta() else { return None };
	(serve.drop_meta().name() == SERVE_WORD).then(|| (port.clone(), routes_in(routes)))
}

/// The routes of a block, one per line (`get "/" {…}` each) or in a row
fn routes_in(items: &[Node]) -> Vec<Route> {
	let words: Vec<Node> = items.iter().flat_map(|item| match item.drop_meta() {
		Node::List(parts, Bracket::None, _) => parts.clone(),
		other => vec![other.clone()],
	}).collect();
	words.chunks(3).filter_map(|route| match route {
		[method, path, body] if METHODS.contains(&method.drop_meta().name().as_str()) => Some((method.drop_meta().name().to_uppercase(), path.clone(), body.clone())),
		_ => None,
	}).collect()
}

/// The route functions, then the call that serves them
fn serving(port: Node, routes: Vec<Route>, count: &mut usize) -> Vec<Node> {
	let mut statements = vec![];
	let mut table = vec![];
	for (method, path, body) in routes {
		let function = format!("{ROUTE_PREFIX}{count}");
		*count += 1;
		let request = Node::Key(Box::new(Node::Symbol(REQUEST_WORD.to_string())), Op::Colon, Box::new(Node::Symbol(ANY_TYPE.to_string())));
		let head = Node::List(vec![Node::Symbol(function.clone()), request], Bracket::Round, Separator::None);
		statements.push(Node::Key(Box::new(head), Op::Define, Box::new(body)));
		table.push(Node::List(vec![Node::Text(method), path, Node::Text(function)], Bracket::Square, Separator::Colon));
	}
	let routes = Node::List(table, Bracket::Square, Separator::Colon);
	statements.push(Node::List(vec![Node::Symbol(crate::host::SERVE_ROUTES.to_string()), port, routes], Bracket::Round, Separator::None));
	statements
}
