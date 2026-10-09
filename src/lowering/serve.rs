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
/// The prerender's requests of those values, `[["/rpc/f", [args]] …]`: a server renders a page with their replies (site.rs)
pub const RPC_REQUESTS: &str = "rpc·requests";
/// The mark of a route answering a list in the route table (web_server.rs routes_of): its ø is the JSON array [], not null
pub const LIST_ANSWER: &str = "list";

/// P221: the server functions giving what a route shows, `route·data·0(path)`
const ROUTE_DATA_PREFIX: &str = "route·data·";
const PATH_PARAMETER: &str = "path";
const PAGE_PATH_CALL: &str = "page_path()";
/// The path a served route was asked at, which binds its parameters
const REQUEST_PATH: &str = "request.path";
const PARAMETER_MARK: char = ':';
/// The page's path as a main-level variable, which page·navigated sets anew when the page goes to another path
/// (host-routes.js navigate)
const PAGE_PATH: &str = "page·path";
const PAGE_NAVIGATED: &str = "page·navigated";
/// Without a server, a form of the page asks the program's own routes: page·submitted(request) (worker.js handleSubmit)
const PAGE_SUBMITTED: &str = "page·submitted";
const DATABASE_WORDS: [&str; 2] = ["database", "indexedDB"];
/// `·` of a generated name as written in code (it would parse as a product)
const NAME_DOT: &str = "_dot_";

/// A route as written: its method (upper case), path and body
type Route = (String, Node, Node);

pub fn lower(program: Node) -> Node {
	let program = with_route_data(program);
	if crate::pipeline::is_for_a_page() {
		return without_serving(program);
	}
	let port = crate::pipeline::serving_port();
	let program = if port.is_none() { with_local_routes(program) } else { program };
	let program = match program {
		single if port.is_some() && !matches!(single.drop_meta(), Node::List(_, Bracket::None, Separator::Semicolon | Separator::Newline)) => {
			Node::List(vec![single], Bracket::None, Separator::Newline)
		}
		other => other,
	};
	match program {
		Node::List(statements, bracket, separator) if port.is_some() || statements.iter().any(|statement| served(statement).is_some() || server_definition(statement).is_some()) => {
			let servers = server_names(&statements);
			let statements: Vec<Node> = statements.into_iter().map(|statement| server_definition(&statement).unwrap_or_else(|| assigned_asking(statement, &servers))).collect();
			let calls: Vec<Route> = statements.iter().filter_map(rpc_route).collect();
			let server_data = ServerData::of(&statements);
			let mut routes = 0;
			let serves_itself = statements.iter().any(|statement| served(statement).is_some());
			// `warp serve`: the top-level routes and the server functions at its port, after the program's statements
			let (top_level, statements): (Vec<Node>, Vec<Node>) = statements.into_iter().partition(|statement| !serves_itself && port.is_some() && !top_level_routes(statement).is_empty());
			let mut statements: Vec<Node> = statements.into_iter().flat_map(|statement| match served(&statement) {
				Some((port, mut routed)) => {
					routed.extend(calls.iter().cloned());
					serving(port, routed, &server_data, &mut routes)
				}
				None => vec![statement],
			}).collect();
			if let (Some(port), false) = (port, serves_itself) {
				let routed = top_level.iter().flat_map(top_level_routes).chain(calls).collect();
				statements.extend(serving(Node::int(i64::from(port)), routed, &server_data, &mut routes));
			}
			Node::List(statements, bracket, separator)
		}
		single if served(&single).is_some() => {
			let (port, routed) = served(&single).expect("checked");
			Node::List(serving(port, routed, &ServerData::default(), &mut 0), Bracket::None, Separator::Semicolon)
		}
		other => other,
	}
}

/// Whether the program is obviously a server, without running it: it serves (`serve PORT {…}`), defines a server
/// function, a page route (`route "/" {…}`) or a top-level `get`/`post` route. Plain `warp app.warp` serves such a
/// program as `warp serve` does (P222)
pub fn serves(code: &str) -> bool {
	top_level_statements(code).iter().any(|statement| served(statement).is_some() || server_definition(statement).is_some() || crate::routes::is_route(statement) || !top_level_routes(statement).is_empty())
}

/// Whether the program has its own `serve PORT {…}`
pub fn serves_itself(code: &str) -> bool {
	top_level_statements(code).iter().any(|statement| served(statement).is_some())
}

/// The program's statements, read quietly: running it says its hints and warnings
fn top_level_statements(code: &str) -> Vec<Node> {
	match crate::diagnostic::quietly(|| crate::warp_parser::parse(code)).drop_meta() {
		Node::List(statements, Bracket::None, Separator::Semicolon | Separator::Newline) => statements.clone(),
		single => vec![single.clone()],
	}
}

/// `serve PORT {…}` or a top-level `get "/api" {…}`: the server's alone, the page leaves it out
fn is_serving(statement: &Node) -> bool {
	served(statement).is_some() || !top_level_routes(statement).is_empty()
}

/// The route a top-level `get "/api" {…}` statement is
fn top_level_routes(statement: &Node) -> Vec<Route> {
	match statement.drop_meta() {
		Node::List(_, Bracket::None, Separator::Space) => routes_in(std::slice::from_ref(statement)),
		_ => vec![],
	}
}

/// P221: a route whose block reads server data asks the server for what it shows. Each value of the block reading it
/// (`"User " + users#id.name` of `h1{…}`) is the server function `route·data·N(path)`, which binds the route's
/// parameters from the path and gives that value (ø for a path of another route); the block calls it with the
/// main-level `page·path` (page_path(), set anew by page·navigated when the page goes to another path), so the page asks
/// POST /rpc/route·data·N anew for each path (asking_the_server). The shipped page leaves the server data out
fn with_route_data(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program else { return program };
	let server_data: Vec<String> = statements.iter().filter_map(server_variable).collect();
	let reads_server_data = |node: &Node| node.mentions_any(&server_data.iter().map(String::as_str).collect::<Vec<_>>());
	if server_data.is_empty() || !statements.iter().any(|statement| crate::routes::route(statement).is_some_and(|(_, body)| body.iter().any(reads_server_data))) {
		return Node::List(statements, bracket, separator);
	}
	let mut data_functions = vec![];
	let mut lowered: Vec<Node> = vec![parse_named(&format!("{PAGE_PATH} = page_path()")), parse_named(&format!("{PAGE_NAVIGATED}() := {{ global {PAGE_PATH}; {PAGE_PATH} = page_path() }}"))];
	for statement in statements {
		match crate::routes::route(&statement) {
			Some((pattern, body)) if body.iter().any(reads_server_data) => {
				let body = body.into_iter().map(|item| data_read(item, &pattern, &reads_server_data, &mut data_functions)).collect();
				let Node::List(items, route_bracket, route_separator) = statement.drop_meta().clone() else { unreachable!("a route is a list") };
				let block = Node::List(body, Bracket::Curly, Separator::Newline);
				lowered.push(Node::List(vec![items[0].clone(), items[1].clone(), block], route_bracket, route_separator));
			}
			_ => lowered.push(statement),
		}
	}
	// where the first route stands, after the server data they read (as routes.rs places its functions)
	let first_route = lowered.iter().position(crate::routes::is_route).unwrap_or(lowered.len());
	lowered.splice(first_route..first_route, data_functions);
	// the shipped page asks the server, so the server data's definitions stay on the server
	if crate::pipeline::is_for_a_page() && !crate::pipeline::is_prerendering() {
		let page_reads = |name: &String| lowered.iter().filter(|statement| server_variable(statement).is_none() && server_definition(statement).is_none() && !is_serving(statement)).any(|statement| statement.mentions_any(&[name]));
		let unread: Vec<String> = server_data.iter().filter(|name| !page_reads(name)).cloned().collect();
		let left_out = |statement: &Node| server_variable(statement).is_some_and(|name| unread.contains(&name));
		let row_classes: Vec<String> = lowered.iter().filter(|statement| left_out(statement)).filter_map(table_class).collect();
		lowered.retain(|statement| !left_out(statement));
		lowered = lowered.into_iter().map(|statement| crate::database_tables::with_row_ids(statement, &row_classes)).collect();
	}
	Node::List(lowered, bracket, separator)
}

/// `stored todos: [Todo]` or `todos: [Todo] = database.todos`: the class of the table's rows
fn table_class(statement: &Node) -> Option<String> {
	let declared = match statement.drop_meta() {
		Node::Key(target, Op::Assign, _) => target.drop_meta().clone(),
		Node::List(items, _, _) => items.last()?.drop_meta().clone(),
		_ => return None,
	};
	let Node::Key(_, Op::Colon, list_type) = declared else { return None };
	match list_type.drop_meta() {
		Node::List(element, Bracket::Square, _) if element.len() == 1 => Some(element[0].drop_meta().name()),
		_ => None,
	}
}

/// `users: [User] = database.users`, `prefs = database.prefs` (indexedDB too), `stored users: [User]`: the variable
/// holding server data
fn server_variable(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::Key(target, Op::Assign, source) => {
			let Node::Key(store, Op::Dot, _) = source.drop_meta() else { return None };
			if !DATABASE_WORDS.contains(&store.drop_meta().name().as_str()) {
				return None;
			}
			match target.drop_meta() {
				Node::Key(name, Op::Colon, _) => Some(name.drop_meta().name()),
				Node::Symbol(name) => Some(name.clone()),
				_ => None,
			}
		}
		Node::List(items, _, _) => match items.as_slice() {
			[word, declaration] if word.drop_meta().name() == crate::stored_values::STORED_WORD => match declaration.drop_meta() {
				Node::Key(name, Op::Colon, list_type) if matches!(list_type.drop_meta(), Node::List(_, Bracket::Square, _)) => Some(name.drop_meta().name()),
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

/// `users: [User] = database.users` or `stored users: [User]`: the table variable and `database.users`, its table.
/// A served route reads it anew at each request (`global users; users = database.users`, reopened by
/// database_tables.rs): a page render runs in an instance of its own and may have changed the table since
fn table_variable(statement: &Node) -> Option<(String, Node)> {
	let name = server_variable(statement)?;
	match statement.drop_meta() {
		Node::Key(target, Op::Assign, source) if matches!(target.drop_meta(), Node::Key(_, Op::Colon, list_type) if matches!(list_type.drop_meta(), Node::List(_, Bracket::Square, _))) => {
			Some((name, source.drop_meta().clone()))
		}
		Node::List(..) => {
			let table = Node::Key(Box::new(Node::Symbol(DATABASE_WORDS[0].to_string())), Op::Dot, Box::new(Node::Symbol(name.clone())));
			Some((name, table))
		}
		_ => None,
	}
}

/// The server's tables and the words giving a list: the table variables and the functions whose value is one
#[derive(Default)]
struct ServerData {
	tables: Vec<(String, Node)>,
	list_words: Vec<String>,
}

impl ServerData {
	fn of(statements: &[Node]) -> ServerData {
		let tables: Vec<(String, Node)> = statements.iter().filter_map(table_variable).collect();
		let mut list_words: Vec<String> = tables.iter().map(|(name, _)| name.clone()).collect();
		let giving_lists: Vec<String> = statements.iter().filter_map(|statement| match statement.drop_meta() {
			Node::Key(_, Op::Define, body) if answers_a_list(body, &list_words) => defined_function(statement).map(|(name, _)| name),
			_ => None,
		}).collect();
		list_words.extend(giving_lists);
		ServerData { tables, list_words }
	}
}

/// Whether a value (a block's last statement) is a list: a list word, its call, a list literal or a filter (`todos where
/// not done`, a comprehension by now), so its ø answers []
fn answers_a_list(value: &Node, list_words: &[String]) -> bool {
	match value.drop_meta() {
		Node::List(items, Bracket::Curly, _) | Node::List(items, Bracket::Round, Separator::Semicolon | Separator::Newline) => items.last().is_some_and(|last| answers_a_list(last, list_words)),
		Node::Symbol(name) => list_words.contains(name) || name.starts_with(crate::comprehensions::MADE),
		Node::List(items, Bracket::Round, _) => items.first().is_some_and(|word| matches!(word.drop_meta(), Node::Symbol(name) if list_words.contains(name))),
		Node::List(_, Bracket::Square, _) => true,
		_ => false,
	}
}

/// The route's body after the statements reading the program's tables anew
/// `post "/todos/:id:int/toggle" {…}`, at the top level or in `serve PORT {…}`, with its path's parameters bound;
/// before lower_where reads the block's variables (a table's filter sends `id` to SQL as a value, card todo-app)
pub fn bind_path_parameters(program: Node) -> Node {
	match program {
		Node::List(statements, bracket @ Bracket::None, separator @ (Separator::Newline | Separator::Semicolon)) => Node::List(statements.into_iter().map(with_bound_routes).collect(), bracket, separator),
		single => with_bound_routes(single),
	}
}

/// The statement's routes with their path's parameters bound
fn with_bound_routes(statement: Node) -> Node {
	let Node::List(items, bracket, separator) = statement.drop_meta().clone() else { return statement };
	match served(&statement).is_some() {
		true => {
			let [serve, port, block] = <[Node; 3]>::try_from(items).expect("served");
			let Node::List(routes, block_bracket, block_separator) = block.drop_meta().clone() else { unreachable!("served") };
			let routes = routes.into_iter().map(|route| match route.drop_meta() {
				Node::List(words, Bracket::None, words_separator) => Node::List(bound_routes(words.clone()), Bracket::None, words_separator.clone()),
				_ => route,
			}).collect();
			Node::List(vec![serve, port, Node::List(bound_routes(routes), block_bracket, block_separator)], bracket, separator)
		}
		false if !top_level_routes(&statement).is_empty() => Node::List(bound_routes(items), bracket, separator),
		false => statement,
	}
}

/// `get "/a/:id" {…} post …` as words, each block with its path's parameters bound
fn bound_routes(words: Vec<Node>) -> Vec<Node> {
	let mut bound = words.clone();
	let mut index = 0;
	while index + 2 < bound.len() {
		match METHODS.contains(&bound[index].drop_meta().name().as_str()) {
			true => {
				bound[index + 2] = with_path_parameters(&bound[index + 1], bound[index + 2].clone());
				index += 3;
			}
			false => index += 1,
		}
	}
	bound
}

/// The route's block with its path's parameters bound from the request's path, as a page's route binds them from
/// page_path() (routes.rs route_body); web_server.rs path_fits picks the route
fn with_path_parameters(path: &Node, body: Node) -> Node {
	let pattern = match path.drop_meta() {
		Node::Text(text) => text.clone(),
		_ => return body,
	};
	if !pattern.contains(PARAMETER_MARK) {
		return body;
	}
	let items = match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.clone(),
		_ => vec![body],
	};
	let (page_path, request_path) = (crate::warp_parser::parse(PAGE_PATH_CALL), crate::warp_parser::parse(REQUEST_PATH));
	let bound = crate::routes::route_body(&pattern, &items).into_iter().map(|item| replaced(item, &page_path, &request_path));
	Node::List(bound.collect(), Bracket::Curly, Separator::Newline)
}

fn reading_tables(body: Node, tables: &[(String, Node)]) -> Node {
	if tables.is_empty() {
		return body;
	}
	let reads = tables.iter().flat_map(|(name, table)| [global(name), Node::Key(Box::new(Node::Symbol(name.clone())), Op::Assign, Box::new(table.clone()))]);
	prepended(reads.collect(), body)
}

/// `global name`
fn global(name: &str) -> Node {
	crate::warp_parser::parse(&format!("{} {name}", crate::late_binding::GLOBAL))
}

/// The block with these statements first
fn prepended(first: Vec<Node>, body: Node) -> Node {
	let statements = match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.clone(),
		_ => vec![body],
	};
	Node::List(first.into_iter().chain(statements).collect(), Bracket::Curly, Separator::Newline)
}

/// The item with each value reading server data as the call of a server function giving it; markup and blocks are
/// looked into, any other value is asked for whole
fn data_read(node: Node, pattern: &str, reads_server_data: &dyn Fn(&Node) -> bool, functions: &mut Vec<Node>) -> Node {
	if !reads_server_data(&node) {
		return node;
	}
	if is_markup(&node) {
		return node.map_children(|child| data_read(child, pattern, reads_server_data, functions));
	}
	let name = format!("{ROUTE_DATA_PREFIX}{}", functions.len());
	let path = Node::Symbol(PATH_PARAMETER.to_string());
	let page_path = crate::warp_parser::parse(PAGE_PATH_CALL);
	let body: Vec<Node> = crate::routes::route_body(pattern, &[node]).into_iter().map(|item| replaced(item, &page_path, &path)).collect();
	let guard = crate::warp_parser::parse(&format!("if not route_matches({pattern:?}, {PATH_PARAMETER}) {{ return ø }}"));
	let head = Node::List(vec![Node::Symbol(name.clone()), Node::Key(Box::new(path), Op::Colon, Box::new(Node::Symbol("text".to_string())))], Bracket::Round, Separator::None);
	let definition = Node::Key(Box::new(head), Op::Define, Box::new(Node::List([vec![guard], body].concat(), Bracket::Curly, Separator::Newline)));
	functions.push(Node::List(vec![Node::Symbol(SERVER_WORD.to_string()), definition], Bracket::None, Separator::Space));
	Node::List(vec![Node::Symbol(name), Node::Symbol(PAGE_PATH.to_string())], Bracket::Round, Separator::None)
}

/// `h1{…}`, `{…}`: markup or a block, whose items are looked into
fn is_markup(node: &Node) -> bool {
	match node.drop_meta() {
		Node::List(_, Bracket::Curly, _) => true,
		Node::List(items, _, _) => items.last().is_some_and(|last| matches!(last.drop_meta(), Node::List(_, Bracket::Curly, _))) && items.first().is_some_and(|first| matches!(first.drop_meta(), Node::Symbol(_))),
		Node::Key(tag, _, block) => matches!(tag.drop_meta(), Node::Symbol(_)) && matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)),
		_ => false,
	}
}

/// The node with each occurrence of `old` as `new`
fn replaced(node: Node, old: &Node, new: &Node) -> Node {
	match node.serialize() == old.serialize() {
		true => new.clone(),
		false => node.map_children(|child| replaced(child, old, new)),
	}
}

/// The code with its generated names (`page·path`, written so it parses as a name)
fn parse_named(code: &str) -> Node {
	with_generated_names(crate::warp_parser::parse(&code.replace('·', NAME_DOT)))
}

fn with_generated_names(node: Node) -> Node {
	match node {
		Node::Symbol(name) if name.contains(NAME_DOT) => Node::Symbol(name.replace(NAME_DOT, "·")),
		other => other.map_children(with_generated_names),
	}
}

/// Without a server (`warp run`, the playground): each top-level route (`post "/todos" {…}`) is the function
/// route·N(request), as served, and page·submitted(request) calls the one answering the request of a form of the page.
/// Like an event handler, a route changes the page's variables: those it mentions are its globals
fn with_local_routes(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program else { return program };
	let Some(first_route) = statements.iter().position(|statement| !top_level_routes(statement).is_empty()) else {
		return Node::List(statements, bracket, separator);
	};
	let (routed, mut statements): (Vec<Node>, Vec<Node>) = statements.into_iter().partition(|statement| !top_level_routes(statement).is_empty());
	let mut page_variables = crate::event_signals::main_level_variables(&statements);
	page_variables.extend(statements.iter().filter_map(server_variable));
	let mut functions = vec![];
	let mut answers = vec![];
	for (index, (method, path, body)) in routed.iter().flat_map(top_level_routes).enumerate() {
		let function = format!("{ROUTE_PREFIX}{index}");
		let globals = page_variables.iter().filter(|name| body.mentions_any(&[name.as_str()])).map(|name| global(name)).collect();
		functions.push(route_function(&function, prepended(globals, body)));
		answers.push(format!("if request.method == {method:?} and route_matches({}, request.path) {{ return {function}(request) }}", path.serialize()));
	}
	let unanswered = "throw \"no route answers \" + request.method + \" \" + request.path";
	functions.push(parse_named(&format!("{PAGE_SUBMITTED}(request:any) := {{\n{}\n{unanswered}\n}}", answers.join("\n"))));
	// where the first route stands: a page route written last stays the program's value
	statements.splice(first_route..first_route, functions);
	Node::List(statements, bracket, separator)
}

/// `route·N(request:any) := body`
fn route_function(name: &str, body: Node) -> Node {
	let request = Node::Key(Box::new(Node::Symbol(REQUEST_WORD.to_string())), Op::Colon, Box::new(Node::Symbol(ANY_TYPE.to_string())));
	let head = Node::List(vec![Node::Symbol(name.to_string()), request], Bracket::Round, Separator::None);
	Node::Key(Box::new(head), Op::Define, Box::new(body))
}

/// A program compiled for its page (pipeline::for_a_page) does not serve: the server runs it natively, the page in the
/// browser (notes/web_framework.md "Built sites"). Its calls of server functions ask the server (asking_the_server)
fn without_serving(program: Node) -> Node {
	match program {
		Node::List(statements, bracket, separator) => {
			let statements: Vec<Node> = statements.into_iter().filter(|statement| !is_serving(statement)).collect();
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
		let requests = Node::List((0..calls.len()).map(|index| rpc_request(&asking(index).0)).collect(), Bracket::Square, Separator::Colon);
		let globals: HashSet<String> = main_variables.into_iter().chain(names).collect();
		// first: the last statement shows the page
		kept.insert(0, crate::event_signals::function_with_globals(RPC_VALUES, false, &[values], &globals));
		kept.insert(0, crate::event_signals::function_with_globals(RPC_REQUESTS, false, &[requests], &globals));
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
	let arguments = (1..=parameters).map(|index| crate::warp_parser::parse(&format!("{REQUEST_WORD}.body#{index}")));
	// built, not parsed: a generated name's `·` would parse as a product
	let call = Node::List(std::iter::once(Node::Symbol(name.clone())).chain(arguments).collect(), Bracket::Round, Separator::None);
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
fn serving(port: Node, routes: Vec<Route>, server_data: &ServerData, count: &mut usize) -> Vec<Node> {
	let mut statements = vec![];
	let mut table = vec![];
	for (method, path, body) in routes {
		let function = format!("{ROUTE_PREFIX}{count}");
		*count += 1;
		let mut entry = vec![Node::Text(method), path, Node::Text(function.clone())];
		if answers_a_list(&body, &server_data.list_words) {
			entry.push(Node::Text(LIST_ANSWER.to_string()));
		}
		statements.push(route_function(&function, reading_tables(body, &server_data.tables)));
		table.push(Node::List(entry, Bracket::Square, Separator::Colon));
	}
	let routes = Node::List(table, Bracket::Square, Separator::Colon);
	statements.push(Node::List(vec![Node::Symbol(crate::host::SERVE_ROUTES.to_string()), port, routes], Bracket::Round, Separator::None));
	statements
}
