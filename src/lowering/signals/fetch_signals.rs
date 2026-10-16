//! Async data (card web-async, notes/web_framework.md "Async data"): `users := fetch "/api/users"` at the main level
//! is a value that arrives later. It starts as ø with `users.loading` true and `users.error` ø, the fetch goes on
//! without the program waiting (fetch_start, src/web/fetches.rs natively, web/playground/host.js in the browser), and the
//! handler on·fetch·0 sets `users.loading` false, `users.error` and `users` (the parsed JSON, else the text) once the
//! reply arrived, so `on change users {…}` and the page's markup see it. A URL reading main-level variables is fetched
//! anew when one of them changes (`on change id {…}`). `users.loading` and `users.error` are the variables
//! users·loading and users·error.
//! The outermost element showing a fetched value is a live region, `aria-live: "polite"` (card web-a11y-live): a screen
//! reader announces the value when it arrives; an element setting aria-live itself keeps its own.

use crate::element_events::{element_items, with_element_items};
use crate::event_signals::{function_with_globals, main_level_variables};
use crate::host::FETCH_WORD;
use crate::node::{symbol, Node};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const LOADING_FIELD: &str = "loading";
const ERROR_FIELD: &str = "error";
const START_TEMPLATE: &str = "VALUE = DEFAULT; LOADING = true; ERROR = ø; fetch_start(ID, URL)";
/// The value written last: a listener on it sees loading and error already set
const ARRIVED_TEMPLATE: &str = "REPLY = fetch_reply(ID); LOADING = false; ERROR = REPLY#2; VALUE = ARRIVED";
const ARRIVED_VALUE: &str = "REPLY#1";
/// `users := fetch url ?? []`: the default until the reply is in, and in place of a failed one
const ARRIVED_OR_DEFAULT: &str = "REPLY#1 ?? DEFAULT";
const REFETCH_TEMPLATE: &str = "on change SOURCE { LOADING = true; fetch_start(ID, URL) }";
const REPLY_FIELD: &str = "reply";
const LIVE_ATTRIBUTE: &str = "aria-live";
const POLITE: &str = "polite";

pub fn lower(program: Node) -> Node {
	if !program.mentions_any(&[FETCH_WORD]) {
		return program;
	}
	let (statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	let fetched: Vec<Fetched> = statements.iter().filter_map(async_fetch).collect();
	if fetched.is_empty() {
		return program;
	}
	let main_variables: HashSet<String> = main_level_variables(&statements)
		.into_iter()
		.chain(fetched.iter().flat_map(|fetched| {
			[fetched.name.clone(), generated(&fetched.name, LOADING_FIELD), generated(&fetched.name, ERROR_FIELD)]
		}))
		.collect();
	let mut count = 0;
	let mut lowered = vec![];
	for statement in statements {
		let Some(Fetched { name, url, default }) = async_fetch(&statement) else {
			lowered.push(statement);
			continue;
		};
		let bindings = bindings(&name, &url, default, count);
		let handler = format!("{}{count}", crate::host::FETCH_HANDLER_PREFIX);
		lowered.push(function_with_globals(
			&handler,
			false,
			&instantiated(ARRIVED_TEMPLATE, &bindings),
			&main_variables,
		));
		lowered.extend(instantiated(START_TEMPLATE, &bindings));
		let mut sources: Vec<String> = crate::variable_signals::symbols(&url)
			.into_iter()
			.filter(|symbol| main_variables.contains(symbol))
			.collect();
		sources.dedup();
		for source in sources {
			let bindings: HashMap<String, Node> =
				bindings.clone().into_iter().chain([("SOURCE".to_string(), Node::Symbol(source))]).collect();
			lowered.extend(instantiated(REFETCH_TEMPLATE, &bindings));
		}
		count += 1;
	}
	let names: Vec<String> = fetched.into_iter().map(|fetched| fetched.name).collect();
	let lowered = lowered.into_iter().map(|statement| with_live_regions(statement, &names)).collect();
	with_state_variables(Node::List(lowered, bracket, separator), &names)
}

/// The outermost elements whose own content (not a child element's) reads a fetched value, marked live
fn with_live_regions(node: Node, names: &[String]) -> Node {
	let names_read: Vec<&str> = names.iter().map(String::as_str).collect();
	let shows_fetched = element_items(&node).is_some_and(|items| {
		items.iter().any(|item| element_items(item.drop_meta()).is_none() && item.mentions_any(&names_read))
	});
	if !shows_fetched {
		return node.map_children(|child| with_live_regions(child, names));
	}
	with_element_items(node, |items| {
		let sets_live = items
			.iter()
			.any(|item| matches!(item.drop_meta(), Node::Key(name, Op::Colon, _) if name.is_symbol(LIVE_ATTRIBUTE)));
		let live = (!sets_live)
			.then(|| Node::Key(Box::new(symbol(LIVE_ATTRIBUTE)), Op::Colon, Box::new(Node::Text(POLITE.to_string()))));
		live.into_iter().chain(items).collect()
	})
}

struct Fetched {
	name: String,
	url: Node,
	default: Option<Node>,
}

/// `users := fetch url` or `users := fetch url ?? default` (a fetch with a timeout stays a fetch on each read)
fn async_fetch(statement: &Node) -> Option<Fetched> {
	let Node::Key(target, Op::Define, value) = statement.drop_meta() else {
		return None;
	};
	let name = target.symbol_name()?;
	let (fetch, default) = match value.drop_meta() {
		Node::Key(fetch, Op::Coalesce, default) => (fetch.as_ref(), Some(default.as_ref().clone())),
		fetch => (fetch, None),
	};
	match crate::host::fetch_call(fetch)? {
		(url, None) => Some(Fetched { name: name.to_string(), url, default }),
		_ => None,
	}
}

/// users·loading
fn generated(name: &str, field: &str) -> String {
	format!("{name}·{field}")
}

fn bindings(name: &str, url: &Node, default: Option<Node>, id: usize) -> HashMap<String, Node> {
	let symbol = |text: String| Node::Symbol(text);
	let reply = HashMap::from([
		("REPLY".to_string(), symbol(generated(name, REPLY_FIELD))),
		("DEFAULT".to_string(), default.clone().unwrap_or(Node::Empty)),
	]);
	let arrived = crate::law::substitute(
		crate::warp_parser::parse_generated(if default.is_some() { ARRIVED_OR_DEFAULT } else { ARRIVED_VALUE })
			.drop_meta(),
		&reply,
	);
	HashMap::from([
		("DEFAULT".to_string(), default.unwrap_or(Node::Empty)),
		("ARRIVED".to_string(), arrived),
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
	let (statements, _, _) = crate::variable_signals::main_statements(&crate::warp_parser::parse_generated(template));
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
