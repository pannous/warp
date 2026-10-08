//! `serve 8080 { get "/api/users" { users } post "/echo" { request.body } }` (card web-server): each route becomes the
//! function `route·N(request:any) := body` and the statement the host call `serve_routes(8080, [["GET",
//! "/api/users", "route·0"] …])`, which serves until it is stopped (src/web_server.rs natively; the playground says
//! loudly that it cannot serve). `server def f(a, b) {…}` (typed RPC, notes/web_framework.md "Server and page") is f,
//! and also the route POST /rpc/f calling it with the request's JSON array of arguments. A program file whose last line
//! shows a page serves that page too, at / (src/site.rs); its page build leaves the serving out and asks the server
//! for a server function's value (without_serving).

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

const SERVE_WORD: &str = "serve";
const ROUTE_PREFIX: &str = "route·";
const REQUEST_WORD: &str = "request";
const ANY_TYPE: &str = "any";
const METHODS: [&str; 5] = ["get", "post", "put", "delete", "patch"];
const SERVER_WORD: &str = "server";
const RPC_PREFIX: &str = "/rpc/";
const RPC_METHOD: &str = "POST";
/// `f·rpc·0`: the value of the page's first call of the server function f
const RPC_VALUE_INFIX: &str = "·rpc·";
/// What a prerendered page exports: the values its calls of server functions gave, the shipped page's first ones
pub const RPC_VALUES: &str = "rpc·values";

/// A route as written: its method (upper case), path and body
type Route = (String, Node, Node);

pub fn lower(program: Node) -> Node {
	if crate::pipeline::is_for_a_page() {
		return without_serving(program);
	}
	match program {
		Node::List(statements, bracket, separator) if statements.iter().any(|statement| served(statement).is_some() || server_definition(statement).is_some()) => {
			let servers = server_names(&statements);
			let statements: Vec<Node> = statements.into_iter().map(|statement| server_definition(&statement).unwrap_or_else(|| assigned_asking(statement, &servers))).collect();
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
/// browser (notes/web_framework.md "Built sites"). Its calls of server functions ask the server (asking_the_server)
fn without_serving(program: Node) -> Node {
	match program {
		Node::List(statements, bracket, separator) => {
			let statements: Vec<Node> = statements.into_iter().filter(|statement| served(statement).is_none()).collect();
			let servers = server_names(&statements);
			if servers.is_empty() {
				return Node::List(statements, bracket, separator);
			}
			asking_the_server(statements, &servers).map_or_else(|error| error, |statements| Node::List(statements, bracket, separator))
		}
		single if served(&single).is_some() => Node::Empty,
		other => other,
	}
}

/// Each call of a server function in the page read from a main-level variable of its own, `f·rpc·0`. The shipped page
/// leaves the server functions out and fetches it, `f·rpc·0 := fetch ["/rpc/f", [args]] ?? first value`, anew when an
/// argument changes (lowering/fetch_signals.rs); its first value is what the call gave the prerender
/// (pipeline::prerendering), which keeps the functions and calls them, and exports the values as rpc·values (site.rs)
fn asking_the_server(statements: Vec<Node>, servers: &[String]) -> Result<Vec<Node>, Node> {
	let page_code: Vec<Node> = statements.iter().filter(|statement| server_definition(statement).is_none()).cloned().collect();
	let calls = server_calls(&page_code, servers);
	// a page calling no server function needs none of them, prerendered or not
	let prerendering = crate::pipeline::is_prerendering() && !calls.is_empty();
	let main_variables = crate::event_signals::main_level_variables(&page_code);
	let functions: HashSet<String> = statements.iter().filter_map(|statement| defined_function(&server_definition(statement).unwrap_or_else(|| statement.clone()))).map(|(name, _)| name).collect();
	if let Some(error) = calls.iter().find_map(|call| local_argument(call, &main_variables, &functions)) {
		return Err(error);
	}
	// `g := f(x)` names the value of its call g
	let names: Vec<String> = calls.iter().enumerate().map(|(index, call)| page_code.iter().find_map(|statement| defined_as(statement, call)).unwrap_or_else(|| format!("{}{RPC_VALUE_INFIX}{index}", callee(call)))).collect();
	let read = |node: Node| with_calls_read(node, &calls, &names);
	let asking = |index: usize| {
		let call = Node::List(call_items(&calls[index]).into_iter().map(&read).collect(), Bracket::Round, Separator::None);
		let value = if prerendering {
			call.clone()
		} else {
			let fetch = Node::List(vec![Node::Symbol(crate::host::FETCH_WORD.to_string()), rpc_request(&call)], Bracket::None, Separator::Space);
			Node::Key(Box::new(fetch), Op::Coalesce, Box::new(crate::pipeline::server_value(index)))
		};
		let op = if prerendering { Op::Assign } else { Op::Define };
		(call, Node::Key(Box::new(Node::Symbol(names[index].clone())), op, Box::new(value)))
	};
	let mut kept = vec![];
	let mut asked_in_place = HashSet::new();
	for statement in statements {
		match server_definition(&statement) {
			Some(definition) if prerendering => kept.push(definition),
			Some(_) => {}
			None => match calls.iter().position(|call| defined_as(&statement, call).is_some()) {
				Some(index) => {
					asked_in_place.insert(index);
					kept.push(asking(index).1);
				}
				None => kept.push(read(statement)),
			},
		}
	}
	let mut asked: Vec<(usize, usize, Node)> = (0..calls.len()).filter(|index| !asked_in_place.contains(index)).map(|index| {
		let (call, statement) = asking(index);
		(asked_at(&kept, &names[index], &call, &main_variables), index, statement)
	}).collect();
	// the last first: an inner call (a later index) before the call around it
	asked.sort_by_key(|(position, index, _)| std::cmp::Reverse((*position, std::cmp::Reverse(*index))));
	for (position, _, statement) in asked {
		kept.insert(position, statement);
	}
	if prerendering {
		let values = Node::List(names.iter().cloned().map(Node::Symbol).collect(), Bracket::Square, Separator::Colon);
		let globals: HashSet<String> = main_variables.into_iter().chain(names).collect();
		// first: the last statement shows the page
		kept.insert(0, crate::event_signals::function_with_globals(RPC_VALUES, false, &[values], &globals));
	}
	Ok(kept)
}

fn server_names(statements: &[Node]) -> Vec<String> {
	statements.iter().filter_map(server_definition).filter_map(|definition| defined_function(&definition)).map(|(name, _)| name).collect()
}

/// `g := f(x)` of a server function f, the page's asking form, is on the server the value it renders, as in the
/// prerender (asking_the_server): `g = f(x)`
fn assigned_asking(statement: Node, servers: &[String]) -> Node {
	match statement.drop_meta() {
		Node::Key(target, Op::Define, value) if matches!(target.drop_meta(), Node::Symbol(_)) && is_server_call(value, servers) => {
			Node::Key(target.clone(), Op::Assign, value.clone())
		}
		_ => statement,
	}
}

/// `g := f(x)` of the call: g
fn defined_as(statement: &Node, call: &Node) -> Option<String> {
	let Node::Key(target, Op::Define, value) = statement.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	(value.serialize() == call.serialize()).then(|| name.clone())
}

/// The calls of server functions in the page's code, each once, outer before inner
fn server_calls(page_code: &[Node], servers: &[String]) -> Vec<Node> {
	let mut calls: Vec<Node> = vec![];
	for statement in page_code {
		statement.visit(&mut |node| {
			if is_server_call(node, servers) && !calls.iter().any(|call| call.serialize() == node.serialize()) {
				calls.push(node.clone());
			}
		});
	}
	calls
}

/// `f(a, b)` / `f a` of a server function f
fn is_server_call(node: &Node, servers: &[String]) -> bool {
	matches!(node.drop_meta(), Node::List(_, Bracket::Round | Bracket::None, _)) && crate::tuples::call_parts(node).is_some_and(|(name, _)| servers.iter().any(|server| server == name))
}

fn callee(call: &Node) -> String {
	crate::tuples::call_parts(call).map(|(name, _)| name.to_string()).unwrap_or_default()
}

/// The callee and the arguments of a call, `f(a, b)` written either way
fn call_items(call: &Node) -> Vec<Node> {
	let Some((name, arguments)) = crate::tuples::call_parts(call) else { return vec![call.clone()] };
	let arguments = match arguments {
		[single] => match single.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.clone(),
			_ => arguments.to_vec(),
		},
		several => several.to_vec(),
	};
	std::iter::once(Node::Symbol(name.to_string())).chain(arguments).collect()
}

/// `["/rpc/f", [a, b]]`, the request fetch_start POSTs
fn rpc_request(call: &Node) -> Node {
	let items = call_items(call);
	let path = Node::Text(format!("{RPC_PREFIX}{}", callee(call)));
	Node::List(vec![path, Node::List(items[1..].to_vec(), Bracket::Square, Separator::Colon)], Bracket::Square, Separator::Colon)
}

/// Each of the calls in `node` as the variable holding its value
fn with_calls_read(node: Node, calls: &[Node], names: &[String]) -> Node {
	match calls.iter().position(|call| call.serialize() == node.serialize()) {
		Some(index) => Node::Symbol(names[index].clone()),
		None => node.map_children(|child| with_calls_read(child, calls, names)),
	}
}

/// Where the page asks: before the first statement reading the value, after its arguments got their first values
fn asked_at(statements: &[Node], name: &str, call: &Node, main_variables: &HashSet<String>) -> usize {
	let first_read = statements.iter().position(|statement| statement.mentions_any(&[name])).unwrap_or(statements.len());
	let arguments: Vec<String> = crate::variable_signals::symbols(call).into_iter().filter(|symbol| main_variables.contains(symbol)).collect();
	let first_assigned = |variable: &String| statements.iter().position(|statement| crate::event_signals::main_level_variables(std::slice::from_ref(statement)).contains(variable)).map_or(0, |index| index + 1);
	arguments.iter().map(first_assigned).fold(first_read, usize::max)
}

/// A call whose argument names a value only a function of the page knows: the page asks before its code runs
fn local_argument(call: &Node, main_variables: &HashSet<String>, functions: &HashSet<String>) -> Option<Node> {
	let items = call_items(call);
	let mut heads = HashSet::new();
	for argument in &items[1..] {
		argument.visit(&mut |node| if let Some((name, _)) = crate::tuples::call_parts(node) { heads.insert(name.to_string()); });
	}
	let known = |symbol: &String| main_variables.contains(symbol) || functions.contains(symbol) || heads.contains(symbol);
	let local = items[1..].iter().flat_map(crate::variable_signals::symbols).find(|symbol| !known(symbol))?;
	let name = callee(call);
	Some(crate::diagnostic::Diagnostic::at(call, format!("the page asks the server for {name}(…) before its code runs, so it sends main-level values only, not {local}: compute {local} at the main level or call {name} from a function of the server (POST {RPC_PREFIX}{name}, notes/server_routes.md)")).into_error())
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
