//! Async data (card web-async, notes/web_framework.md "Async data"): `users := fetch "/api/users"` at the main level
//! is a value that arrives later. It starts as ø with `users.loading` true and `users.error` ø, the fetch goes on
//! without the program waiting (fetch_start, src/fetches.rs natively, web/playground/host.js in the browser), and the
//! handler on·fetch·0 sets `users.loading` false, `users.error` and `users` (the parsed JSON, else the text) once the
//! reply arrived, so `on change users {…}` and the page's markup see it. A URL reading main-level variables is fetched
//! anew when one of them changes (`on change id {…}`). `users.loading` and `users.error` are the variables
//! users·loading and users·error.

use crate::event_signals::{function_with_globals, main_level_variables};
use crate::node::Node;
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const LOADING_FIELD: &str = "loading";
const ERROR_FIELD: &str = "error";
/// A program without this word holds no async fetch: the pass skips it
const FETCH_WORD: &str = "fetch";
const START_TEMPLATE: &str = "VALUE = ø; LOADING = true; ERROR = ø; fetch_start(ID, URL)";
/// The value written last: a listener on it sees loading and error already set
const ARRIVED_TEMPLATE: &str = "REPLY = fetch_reply(ID); LOADING = false; ERROR = REPLY#2; VALUE = REPLY#1";
const REFETCH_TEMPLATE: &str = "on change SOURCE { LOADING = true; fetch_start(ID, URL) }";
const REPLY_FIELD: &str = "reply";

pub fn lower(program: Node) -> Node {
	if !program.mentions_any(&[FETCH_WORD]) {
		return program;
	}
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	let fetched: Vec<(String, Node)> = statements.iter().filter_map(async_fetch).collect();
	if fetched.is_empty() {
		return program;
	}
	let main_variables: HashSet<String> = main_level_variables(&statements).into_iter()
		.chain(fetched.iter().flat_map(|(name, _)| [name.clone(), generated(name, LOADING_FIELD), generated(name, ERROR_FIELD)]))
		.collect();
	let mut count = 0;
	let mut lowered = vec![];
	for statement in statements {
		let Some((name, url)) = async_fetch(&statement) else {
			lowered.push(statement);
			continue;
		};
		let bindings = bindings(&name, &url, count);
		let handler = format!("{}{count}", crate::host::FETCH_HANDLER_PREFIX);
		lowered.push(function_with_globals(&handler, false, &instantiated(ARRIVED_TEMPLATE, &bindings), &main_variables));
		lowered.extend(instantiated(START_TEMPLATE, &bindings));
		let mut sources: Vec<String> = crate::variable_signals::symbols(&url).into_iter().filter(|symbol| main_variables.contains(symbol)).collect();
		sources.dedup();
		for source in sources {
			let bindings: HashMap<String, Node> = bindings.clone().into_iter().chain([("SOURCE".to_string(), Node::Symbol(source))]).collect();
			lowered.extend(instantiated(REFETCH_TEMPLATE, &bindings));
		}
		count += 1;
	}
	let names: Vec<String> = fetched.into_iter().map(|(name, _)| name).collect();
	with_state_variables(Node::List(lowered, bracket, separator), &names)
}

/// `users := fetch url`: the name and the URL (a fetch with a timeout stays a fetch on each read)
fn async_fetch(statement: &Node) -> Option<(String, Node)> {
	let Node::Key(target, Op::Define, value) = statement.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	match crate::host::fetch_call(value)? {
		(url, None) => Some((name.clone(), url)),
		_ => None,
	}
}

/// users·loading
fn generated(name: &str, field: &str) -> String {
	format!("{name}·{field}")
}

fn bindings(name: &str, url: &Node, id: usize) -> HashMap<String, Node> {
	let symbol = |text: String| Node::Symbol(text);
	HashMap::from([
		("VALUE".to_string(), symbol(name.to_string())),
		("LOADING".to_string(), symbol(generated(name, LOADING_FIELD))),
		("ERROR".to_string(), symbol(generated(name, ERROR_FIELD))),
		("REPLY".to_string(), symbol(generated(name, REPLY_FIELD))),
		("ID".to_string(), Node::int(id as i64)),
		("URL".to_string(), url.clone()),
	])
}

/// The statements of a template with its placeholders bound
fn instantiated(template: &str, bindings: &HashMap<String, Node>) -> Vec<Node> {
	let (statements, _, _) = crate::variable_signals::main_statements(&crate::wasp_parser::parse(template));
	statements.iter().map(|statement| crate::law::substitute(statement.drop_meta(), bindings)).collect()
}

/// `users.loading` and `users.error` anywhere in the program are the variables users·loading and users·error
fn with_state_variables(node: Node, names: &[String]) -> Node {
	if let Node::Key(receiver, Op::Dot, field) = node.drop_meta() {
		if let (Node::Symbol(name), Node::Symbol(field)) = (receiver.drop_meta(), field.drop_meta()) {
			if names.contains(name) && [LOADING_FIELD, ERROR_FIELD].contains(&field.as_str()) {
				return Node::Symbol(generated(name, field));
			}
		}
	}
	node.map_children(|child| with_state_variables(child, names))
}
