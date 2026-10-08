//! `serve 8080 { get "/api/users" { users } post "/echo" { request.body } }` (card web-server): each route becomes the
//! function `route·N(request:any) := body` and the statement the host call `serve_routes(8080, [["GET",
//! "/api/users", "route·0"] …])`, which serves until it is stopped (src/web_server.rs natively; the playground says
//! loudly that it cannot serve). `server def f(a, b) {…}` (typed RPC, notes/web_framework.md "Server and page") is f,
//! and also the route POST /rpc/f calling it with the request's JSON array of arguments. A program file whose last line
//! shows a page serves that page too, at / (src/site.rs); its page build leaves the serving out.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const SERVE_WORD: &str = "serve";
const ROUTE_PREFIX: &str = "route·";
const REQUEST_WORD: &str = "request";
const ANY_TYPE: &str = "any";
const METHODS: [&str; 5] = ["get", "post", "put", "delete", "patch"];
const SERVER_WORD: &str = "server";
const RPC_PREFIX: &str = "/rpc/";
const RPC_METHOD: &str = "POST";

/// A route as written: its method (upper case), path and body
type Route = (String, Node, Node);

pub fn lower(program: Node) -> Node {
	if crate::pipeline::is_for_a_page() {
		return without_serving(program);
	}
	match program {
		Node::List(statements, bracket, separator) if statements.iter().any(|statement| served(statement).is_some() || server_definition(statement).is_some()) => {
			let statements: Vec<Node> = statements.into_iter().map(|statement| server_definition(&statement).unwrap_or(statement)).collect();
			let calls: Vec<Route> = statements.iter().filter_map(rpc_route).collect();
			let mut routes = 0;
			let statements = statements.into_iter().flat_map(|statement| match served(&statement) {
				Some((port, mut routed)) => {
					routed.extend(calls.iter().cloned());
					serving(port, routed, &mut routes)
				}
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

/// A program compiled for its page (pipeline::for_a_page) does not serve: the server runs it natively, the page in the
/// browser (notes/web_framework.md "Built sites"). A server function is a function of the page too, its body run there
/// until the page calls POST /rpc/f instead (notes/server_routes.md, step 2)
fn without_serving(program: Node) -> Node {
	match program {
		Node::List(statements, bracket, separator) => {
			let statements: Vec<Node> = statements.into_iter().filter(|statement| served(statement).is_none()).collect();
			let shipped: Vec<_> = statements.iter().filter(|statement| server_definition(statement).is_some()).map(|statement| {
				crate::diagnostic::Diagnostic::at(statement, "a server function also runs in the page and ships in its app.wasm (until the page calls POST /rpc/, notes/server_routes.md): keep secrets out of it".to_string())
			}).collect();
			if let Err(error) = crate::diagnostic::report(&shipped) {
				return error;
			}
			Node::List(statements.into_iter().map(|statement| server_definition(&statement).unwrap_or(statement)).collect(), bracket, separator)
		}
		single if served(&single).is_some() => Node::Empty,
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

/// `server def f(…) {…}` / `server f(…) := …`: the definition itself
fn server_definition(statement: &Node) -> Option<Node> {
	let Node::List(items, bracket, separator) = statement.drop_meta() else { return None };
	let (word, definition) = items.split_first()?;
	if word.drop_meta().name() != SERVER_WORD || definition.is_empty() {
		return None;
	}
	let definition = match definition {
		[single] => single.clone(),
		several => Node::List(several.to_vec(), bracket.clone(), separator.clone()),
	};
	// one definition form (`def f(a) {…}` is `f(a) := {…}`), its parameters taking any value from the JSON arguments
	let definition = with_any_parameters(crate::declarations::lower_c_functions(definition));
	// only a definition: a call of a word named server stays one
	rpc_route(&definition).map(|_| definition)
}

/// `f(a, b:int) := …` as `f(a:any, b:int) := …`: an RPC's arguments are JSON values of any kind
fn with_any_parameters(definition: Node) -> Node {
	let Node::Key(head, Op::Define, body) = definition.drop_meta().clone() else { return definition };
	let Node::List(items, bracket, separator) = head.drop_meta().clone() else { return definition };
	let any = |parameter: Node| match parameter.drop_meta() {
		Node::Symbol(_) => Node::Key(Box::new(parameter), Op::Colon, Box::new(Node::Symbol(ANY_TYPE.to_string()))),
		_ => parameter,
	};
	let mut items = items.into_iter();
	let head: Vec<Node> = items.next().into_iter().chain(items.map(any)).collect();
	Node::Key(Box::new(Node::List(head, bracket, separator)), Op::Define, body)
}

/// POST /rpc/f of a function marked `server`: f called with the items of the request's JSON array
fn rpc_route(definition: &Node) -> Option<Route> {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, definition);
	let function = context.user_functions.into_values().next()?;
	let arguments: Vec<String> = (1..=function.params.len()).map(|index| format!("{REQUEST_WORD}.body#{index}")).collect();
	let call = crate::warp_parser::parse(&format!("{}({})", function.name, arguments.join(", ")));
	Some((RPC_METHOD.to_string(), Node::Text(format!("{RPC_PREFIX}{}", function.name)), Node::List(vec![call], Bracket::Curly, Separator::None)))
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
