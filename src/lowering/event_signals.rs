//! Named event signals (wiki/signal.md, notes/signals.md phase 2, P110): `raise stop the machine{reason:"…"}` runs the
//! `on stop the machine {…}` handlers of the program in their order, `event` in them is the data (ø without). The
//! handlers of a name become one function `on·stop·the·machine(event)`, defined where the first of them is written and
//! declaring `global` the main-level variables they mention, so a raise inside a function reaches them too; each raise
//! of the name is its call. A raise whose name no handler listens to stays the exception (try catches it).
//! Page events (phase 7): `on click {…}` and `on key {…}` are kept without a raise and get the wrapper
//! `on·click·node([event])` (declarations::with_node_wrappers): the browser playground calls it when the page sees the
//! event (web/playground/worker.js); natively nothing does, which a warning says. System events (`on interrupt`) are
//! kept the same way: the runtime calls their handlers (notes/system_signals.md).

use crate::declarations::{handler_parts, word};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::variable_signals::symbols;
use crate::wasp_parser::parse;
use std::collections::{HashMap, HashSet};

const ON_WORD: &str = "on";
const RAISE_WORD: &str = "raise";
/// `on set x` and `on change x` are variable listeners (variable_signals.rs)
const VARIABLE_LISTENER_WORDS: [&str; 2] = ["set", "change"];
pub const HANDLER_PREFIX: &str = "on·";
const NAME_JOINER: &str = "·";
/// A generated function as written, its name and globals filled in
const FUNCTION_TEMPLATE: &str = "handler(event) := {}";
const PARAMETERLESS_TEMPLATE: &str = "handler() := {}";
const TEMPLATE_NAME: &str = "handler";
const EVENT_WORD: &str = "event";
/// The events of the page that call their handlers from outside the program
pub const PAGE_EVENTS: [&str; 2] = ["click", "key"];
/// The output binding of a program with page events: its last line when that is a name, read anew after each handler
pub const PAGE_VALUE: &str = "page·value";
/// The events the system raises: the runtime calls their handlers (notes/system_signals.md)
pub const SYSTEM_EVENTS: [&str; 2] = ["interrupt", "exit"];

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	let raised = raised_names(&program);
	let handlers: Vec<(usize, String, Node)> = statements.iter().enumerate()
		.filter_map(|(index, statement)| handler(statement).map(|(name, body)| (index, name, body)))
		.filter(|(_, name, _)| raised.contains(name) || PAGE_EVENTS.contains(&name.as_str()) || SYSTEM_EVENTS.contains(&name.as_str()))
		.collect();
	if handlers.is_empty() {
		return program;
	}
	#[cfg(feature = "native")]
	if let Some((index, name, _)) = handlers.iter().find(|(_, name, _)| PAGE_EVENTS.contains(&name.as_str()) && !raised.contains(name)) {
		let warning = format!("on {name}: {name} comes from the playground page; a native run never raises it");
		if let Err(error) = crate::diagnostic::report(&[crate::diagnostic::Diagnostic::at(&statements[*index], warning)]) {
			return error;
		}
	}
	let main_variables = main_level_variables(&statements);
	let mut bodies: HashMap<String, Vec<Node>> = HashMap::new();
	let mut first_handler: HashMap<usize, String> = HashMap::new();
	for (index, name, body) in &handlers {
		if !bodies.contains_key(name) {
			first_handler.insert(*index, name.clone());
		}
		bodies.entry(name.clone()).or_default().push(body.clone());
	}
	let handler_lines: HashSet<usize> = handlers.iter().map(|(index, _, _)| *index).collect();
	let lowered = statements.into_iter().enumerate().filter_map(|(index, statement)| match first_handler.get(&index) {
		Some(name) => Some(function_with_globals(&handler_function_name(name), reads_event(&bodies[name]), &bodies[name], &main_variables)),
		None if handler_lines.contains(&index) => None,
		None => Some(statement),
	});
	let lowered: Vec<Node> = lowered.map(|statement| raises_as_calls(statement, &bodies)).collect();
	let page_handlers: std::collections::BTreeMap<String, usize> = PAGE_EVENTS.iter().filter(|event| bodies.contains_key(**event)).map(|event| (handler_function_name(event), usize::from(reads_event(&bodies[*event])))).collect();
	let mut lowered = lowered;
	if let Some(name) = lowered.last().filter(|_| !page_handlers.is_empty()).and_then(|last| match last.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		_ => None,
	}) {
		let binding = function_with_globals(PAGE_VALUE, false, &[Node::Symbol(name)], &main_variables);
		lowered.insert(lowered.len() - 1, binding);
	}
	crate::declarations::with_node_wrappers(Node::List(lowered, bracket, separator), &page_handlers)
}

/// `on alarm {body}`, `on stop the machine {body}`, `on alarm: body`: the event's name and the body
fn handler(statement: &Node) -> Option<(String, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let (on, rest) = items.split_first()?;
	let (last, words) = rest.split_last()?;
	if word(on) != ON_WORD || words.first().is_some_and(|first| VARIABLE_LISTENER_WORDS.contains(&word(first).as_str())) {
		return None;
	}
	let (last_word, body) = match last.drop_meta() {
		Node::List(_, Bracket::Curly, _) if !words.is_empty() => (None, last.clone()),
		_ => {
			let (event, body) = handler_parts(last)?;
			(Some(event), body)
		}
	};
	let name = event_name(words.iter().chain(last_word.as_ref()))?;
	Some((name, body))
}

/// `raise alarm`, `raise stop the machine{reason:"…"}`: the name and the data (ø without)
fn raise(node: &Node) -> Option<(String, Node)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let (raise, rest) = items.split_first()?;
	let (last, words) = rest.split_last()?;
	if word(raise) != RAISE_WORD {
		return None;
	}
	let (last_word, data) = match last.drop_meta() {
		Node::Key(name, Op::Colon, data) if matches!(data.drop_meta(), Node::List(_, Bracket::Curly, _)) => (name.as_ref().clone(), data.as_ref().clone()),
		_ => (last.clone(), Node::Empty),
	};
	Some((event_name(words.iter().chain([&last_word]))?, data))
}

/// The words of an event, joined by spaces; None unless all are plain words
fn event_name<'a>(words: impl Iterator<Item = &'a Node>) -> Option<String> {
	let words: Vec<String> = words.map(word).collect();
	(!words.is_empty() && words.iter().all(|word| !word.is_empty())).then(|| words.join(" "))
}

fn raised_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |part| if let Some((name, _)) = raise(part) { names.insert(name); });
	names
}

/// The events the lowered program handles and their handler functions, exported under these names:
/// `("stop the machine", "on·stop·the·machine")`. A task forwards a raise of one to the starting thread (warp-d9)
pub fn handled_signals(lowered: &Node) -> Vec<(String, String)> {
	let mut handled = vec![];
	lowered.visit(&mut |part| if let Node::Key(head, Op::Define, _) = part {
		if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			let function = items.first().map(word).unwrap_or_default();
			// a page event's wrapper on·click·node (with_node_wrappers) is no handler of its own
			if let Some(event) = function.strip_prefix(HANDLER_PREFIX).filter(|_| !function.ends_with(crate::declarations::NODE_WRAPPER_SUFFIX)) {
				handled.push((event.replace(NAME_JOINER, " "), function.clone()));
			}
		}
	});
	handled
}

fn handler_function_name(event: &str) -> String {
	format!("{HANDLER_PREFIX}{}", event.replace(' ', NAME_JOINER))
}

/// `on·alarm(event) := { global n; body; body2 }`: the main-level variables the bodies mention are declared global
pub(crate) fn function_with_globals(name: &str, takes_event: bool, bodies: &[Node], main_variables: &HashSet<String>) -> Node {
	let mut mentioned: Vec<String> = bodies.iter().flat_map(symbols).filter(|name| main_variables.contains(name) && name != EVENT_WORD).collect();
	mentioned.sort();
	mentioned.dedup();
	let globals = mentioned.iter().map(|name| parse(&format!("global {name}")));
	let statements: Vec<Node> = globals.chain(bodies.iter().cloned()).collect();
	let template = if takes_event { FUNCTION_TEMPLATE } else { PARAMETERLESS_TEMPLATE };
	let Node::Key(head, op, _) = parse(template).drop_meta().clone() else { unreachable!("the template is a definition") };
	let name = Node::Symbol(name.to_string());
	let head = crate::law::substitute(&head, &HashMap::from([(TEMPLATE_NAME.to_string(), name)]));
	Node::Key(Box::new(head), op, Box::new(Node::List(statements, Bracket::Curly, Separator::Semicolon)))
}

/// Handlers take `event` only when a body reads it: an unread parameter has no type a caller from outside (the page)
/// could pass a value as
fn reads_event(bodies: &[Node]) -> bool {
	bodies.iter().flat_map(symbols).any(|name| name == EVENT_WORD)
}

/// Each `raise name{data}` with handlers is the call `on·name(data)`, `on·name()` when no handler reads `event`
fn raises_as_calls(node: Node, handled: &HashMap<String, Vec<Node>>) -> Node {
	if let Some((name, data)) = raise(&node).filter(|(name, _)| handled.contains_key(name)) {
		let arguments = if reads_event(&handled[&name]) { vec![data] } else { vec![] };
		let call = Node::List([vec![Node::Symbol(handler_function_name(&name))], arguments].concat(), Bracket::Round, Separator::None);
		// `{ raise alarm }` is a block of one statement: it stays a block
		let braced = matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _));
		return if braced { Node::List(vec![call], Bracket::Curly, Separator::Semicolon) } else { call };
	}
	match node {
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| raises_as_calls(item, handled)).collect(), bracket, separator),
		Node::Key(left, op, right) => Node::Key(Box::new(raises_as_calls(*left, handled)), op, Box::new(raises_as_calls(*right, handled))),
		Node::Meta { node, data } => Node::Meta { node: Box::new(raises_as_calls(*node, handled)), data },
		other => other,
	}
}

/// The variables the main level assigns (`n = 0`, `n += 1`)
pub(crate) fn main_level_variables(statements: &[Node]) -> HashSet<String> {
	statements.iter().filter_map(|statement| match statement.drop_meta() {
		Node::Key(target, op, _) if *op == Op::Assign || op.is_compound_assign() => match target.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}).collect()
}
