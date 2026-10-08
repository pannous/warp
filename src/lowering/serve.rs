//! `serve 8080 { get "/api/users" { users } post "/echo" { request.body } }` (card web-server): each route becomes the
//! function `route·N(request:any) := body` and the statement the host call `serve_routes(8080, [["GET",
//! "/api/users", "route·0"] …])`, which serves until it is stopped (src/web_server.rs natively; the playground says
//! loudly that it cannot serve). `server def f(a, b) {…}` (typed RPC, notes/web_framework.md "Server and page") is f,
//! and also the route POST /rpc/f calling it with the request's JSON array of arguments. A program file whose last line
//! shows a page serves that page too, at / (src/site.rs); its page build leaves the serving out and asks the server
//! for a server function's value (without_serving).

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
/// browser (notes/web_framework.md "Built sites"). A main-level `x := f(args)` of a server function asks the server, as
/// `x := fetch` does (POST /rpc/f, lowering/fetch_signals.rs), so f's body stays out of app.wasm; a server function the
/// page calls otherwise still runs there, loudly (notes/server_routes.md)
fn without_serving(program: Node) -> Node {
	match program {
		Node::List(statements, bracket, separator) => {
			let statements: Vec<Node> = statements.into_iter().filter(|statement| served(statement).is_none()).collect();
			let servers: Vec<String> = statements.iter().filter_map(server_definition).filter_map(|definition| defined_function(&definition)).map(|(name, _)| name).collect();
			let asked: Vec<Node> = statements.into_iter().map(|statement| server_call(&statement, &servers).unwrap_or(statement)).collect();
			let page_code = Node::List(asked.iter().filter(|statement| server_definition(statement).is_none()).cloned().collect(), Bracket::None, Separator::None);
			let called_directly = |statement: &Node| server_definition(statement).and_then(|definition| defined_function(&definition)).map(|(name, _)| name).filter(|name| page_code.mentions_any(&[name]));
			let warnings: Vec<_> = asked.iter().filter_map(|statement| called_directly(statement).map(|name| crate::diagnostic::Diagnostic::at(statement, format!("the page calls the server function {name} directly, so its body ships in its app.wasm: `x := {name}(…)` asks the server instead (POST {RPC_PREFIX}{name}, notes/server_routes.md)")))).collect();
			if let Err(error) = crate::diagnostic::report(&warnings) {
				return error;
			}
			let shipped: Vec<Node> = asked.iter().filter(|statement| server_definition(statement).is_none() || called_directly(statement).is_some()).map(|statement| server_definition(statement).unwrap_or_else(|| statement.clone())).collect();
			Node::List(shipped, bracket, separator)
		}
		single if served(&single).is_some() => Node::Empty,
		other => other,
	}
}

/// `x := f(a, b)` of a server function f as `x := fetch ["/rpc/f", [a, b]]`, the request fetch_start POSTs
fn server_call(statement: &Node, servers: &[String]) -> Option<Node> {
	let Node::Key(target, Op::Define, value) = statement.drop_meta() else { return None };
	let (name, arguments) = crate::tuples::call_parts(value)?;
	if !servers.iter().any(|server| server == name) {
		return None;
	}
	let arguments = match arguments {
		[single] => match single.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.clone(),
			_ => arguments.to_vec(),
		},
		several => several.to_vec(),
	};
	let request = Node::List(vec![Node::Text(format!("{RPC_PREFIX}{name}")), Node::List(arguments, Bracket::Square, Separator::Colon)], Bracket::Square, Separator::Colon);
	let fetch = Node::List(vec![Node::Symbol(crate::host::FETCH_WORD.to_string()), request], Bracket::None, Separator::Space);
	Some(Node::Key(target.clone(), Op::Define, Box::new(fetch)))
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
	let (name, parameters) = defined_function(definition)?;
	let arguments: Vec<String> = (1..=parameters).map(|index| format!("{REQUEST_WORD}.body#{index}")).collect();
	let call = crate::warp_parser::parse(&format!("{name}({})", arguments.join(", ")));
	Some((RPC_METHOD.to_string(), Node::Text(format!("{RPC_PREFIX}{name}")), Node::List(vec![call], Bracket::Curly, Separator::None)))
}

/// The name and parameter count of the function a definition defines
fn defined_function(definition: &Node) -> Option<(String, usize)> {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, definition);
	context.user_functions.into_values().next().map(|function| (function.name, function.params.len()))
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
