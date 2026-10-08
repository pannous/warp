//! Block-scoped event handlers (card effect-handlers step 1, notes/effect_handlers.md): `on ask { 42 } in { compute() }`
//! answers `emit ask` while the block runs, in it and in the functions it calls; the innermost handler wins and the
//! handler's value is what emit gives (tail-resumptive, plain calls). Before event_signals: an emit no block handler
//! takes stays the emit, which the program-wide `on ask {…}` answers as before.
//! Handler i of ask is the function `ask·handler·i(event)`; the global `ask_active` names the active one (0: none) and
//! `ask_outer_i` the one active when block i was entered, which an emit inside handler i reaches.

use crate::event_signals::{emit_verbs, emitted, function_with_globals, main_level_variables, reads_event};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::variable_signals::{assign, if_then_else};
use std::collections::BTreeMap;

const ON_WORD: &str = "on";
const IN_WORD: &str = "in";
/// Generated names join their parts with it; a generated variable is a plain word (`ask_active`), as `global` parses it
const JOINER: &str = "·";
/// The variable the handler function runs its body into before it restores the active handler
const RESULT_WORD: &str = "handled";
const SAVED_WORD: &str = "saved";

/// A block handler: its event, its number (unique in the program) and body
struct Handler {
	event: String,
	number: usize,
	body: Node,
}

pub fn lower(program: Node) -> Node {
	let mut handlers = vec![];
	let program = scoped_blocks(program, &mut handlers);
	if handlers.is_empty() {
		return program;
	}
	// a program of one statement is that statement
	let (statements, bracket, separator) = match program.drop_meta().clone() {
		Node::List(statements, bracket @ Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => (statements, bracket, separator),
		statement => (vec![statement], Bracket::None, Separator::Semicolon),
	};
	let mut by_event: BTreeMap<String, Vec<usize>> = BTreeMap::new();
	for handler in &handlers {
		by_event.entry(handler.event.clone()).or_default().push(handler.number);
	}
	let verbs = emit_verbs(&program);
	let initial = by_event.keys().map(|event| assign(&active_variable(event), Node::int(0)))
		.chain(handlers.iter().map(|handler| assign(&outer_variable(&handler.event, handler.number), Node::int(0))));
	let mut main: Vec<Node> = initial.collect();
	let main_variables = main_level_variables(&[main.clone(), statements.clone()].concat());
	for event in by_event.keys() {
		main.push(function_with_globals(&active_function(event), false, &[Node::Symbol(active_variable(event))], &main_variables));
		let restore = assign(&active_variable(event), Node::Symbol(crate::event_signals::EVENT_WORD.to_string()));
		main.push(function_with_globals(&leave_function(event), true, &[restore], &main_variables));
	}
	for handler in &handlers {
		main.push(enter_function(handler, &main_variables));
		main.push(handler_function(handler, &by_event, &verbs, &main_variables));
	}
	let statements = statements.into_iter().map(|statement| dispatched_emits(statement, &by_event, &verbs));
	Node::List(main.into_iter().chain(statements).collect(), bracket, separator)
}

/// The program with each `on ask {…} in {…}` replaced by its block run under handler i, the handlers collected
fn scoped_blocks(node: Node, handlers: &mut Vec<Handler>) -> Node {
	let node = node.map_children(|child| scoped_blocks(child, handlers));
	let Some((event, body, block)) = scoped_handler(&node) else { return node };
	let number = handlers.len() + 1;
	handlers.push(Handler { event: event.clone(), number, body });
	let saved = generated(&[SAVED_WORD, &event_word(&event), &number.to_string()]);
	let value = generated(&[RESULT_WORD, &event_word(&event), &number.to_string()]);
	let run = sequence(vec![
		assign(&saved, call(&enter_name(&event, number), vec![])),
		assign(&value, sequence(statements_of(&block))),
		call(&leave_function(&event), vec![Node::Symbol(saved)]),
		Node::Symbol(value),
	]);
	// `{ on ask {…} in {…} }`, a block of this one statement, stays a block
	match node.drop_meta() {
		Node::List(_, Bracket::Curly, _) => Node::List(vec![run], Bracket::Curly, Separator::Semicolon),
		_ => run,
	}
}

/// `on ask {body} in {block}`: the event, the handler body and the block
fn scoped_handler(node: &Node) -> Option<(String, Node, Node)> {
	let Node::List(items, _, Separator::Space) = node.drop_meta() else { return None };
	let items = phrase_words(items);
	let [on, words @ .., body, in_word, block] = items.as_slice() else { return None };
	let curly = |node: &Node| matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _));
	if crate::declarations::word(on) != ON_WORD || crate::declarations::word(in_word) != IN_WORD || !curly(body) || !curly(block) || words.is_empty() {
		return None;
	}
	let words: Vec<String> = words.iter().map(crate::declarations::word).collect();
	if words.iter().any(String::is_empty) {
		return None;
	}
	Some((words.join(" "), body.clone(), block.clone()))
}

/// The words of a phrase, the groups the parser makes of it (`on (ask {…}) in {…}`, `on ask ({…} in {…})`) undone
fn phrase_words(items: &[Node]) -> Vec<Node> {
	items.iter().flat_map(|item| match item.drop_meta() {
		Node::List(inner, Bracket::None, Separator::Space) => phrase_words(inner),
		_ => vec![item.clone()],
	}).collect()
}

/// `emit ask{…}`: `if ask·active() == 1 then ask·handler·1({…}) else … else emit ask{…}`
fn dispatched_emits(node: Node, by_event: &BTreeMap<String, Vec<usize>>, verbs: &[String]) -> Node {
	if let Some((event, data)) = emitted(&node, verbs) {
		if let Some(numbers) = by_event.get(&event) {
			let active = call(&active_function(&event), vec![]);
			return numbers.iter().rev().fold(node, |otherwise, number| {
				let condition = Node::Key(Box::new(active.clone()), Op::Eq, Box::new(Node::int(*number as i64)));
				let arguments = if matches!(data, Node::Empty) { vec![] } else { vec![data.clone()] };
				sequence(vec![if_then_else(condition, call(&handler_name(&event, *number), arguments), otherwise)])
			});
		}
	}
	node.map_children(|child| dispatched_emits(child, by_event, verbs))
}

/// `ask·enter·i() := { global ask_active, ask_outer_i; ask_outer_i = ask_active; ask_active = i; ask_outer_i }`
fn enter_function(handler: &Handler, main_variables: &std::collections::HashSet<String>) -> Node {
	let (active, outer) = (active_variable(&handler.event), outer_variable(&handler.event, handler.number));
	let body = [assign(&outer, Node::Symbol(active.clone())), assign(&active, Node::int(handler.number as i64)), Node::Symbol(outer)];
	function_with_globals(&enter_name(&handler.event, handler.number), false, &body, main_variables)
}

/// `ask·handler·i(event) := { previous = ask_active; ask_active = ask_outer_i; handled = (body); ask_active = previous;
/// handled }`: while the body runs, an emit of ask reaches the handler outside block i
fn handler_function(handler: &Handler, by_event: &BTreeMap<String, Vec<usize>>, verbs: &[String], main_variables: &std::collections::HashSet<String>) -> Node {
	let active = active_variable(&handler.event);
	let previous = generated(&[SAVED_WORD, "previous"]);
	let result = generated(&[RESULT_WORD, "value"]);
	let body = dispatched_emits(sequence(statements_of(&handler.body)), by_event, verbs);
	let statements = [
		assign(&previous, Node::Symbol(active.clone())),
		assign(&active, Node::Symbol(outer_variable(&handler.event, handler.number))),
		assign(&result, body),
		assign(&active, Node::Symbol(previous)),
		Node::Symbol(result),
	];
	function_with_globals(&handler_name(&handler.event, handler.number), reads_event(&[handler.body.clone()]), &statements, main_variables)
}

/// The statements of a block; `{emit ask}` holds the words of its one statement
fn statements_of(block: &Node) -> Vec<Node> {
	match block.drop_meta() {
		Node::List(words, Bracket::Curly, Separator::Space) if words.len() > 1 => vec![Node::List(words.clone(), Bracket::None, Separator::Space)],
		Node::List(statements, Bracket::Curly, _) => statements.clone(),
		other => vec![other.clone()],
	}
}

/// `(a; b; c)`: the statements run in order, the last one's value is the value
fn sequence(statements: Vec<Node>) -> Node {
	Node::List(statements, Bracket::Round, Separator::Semicolon)
}

fn call(function: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(function.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}

fn generated(parts: &[&str]) -> String {
	parts.join(JOINER)
}

fn event_word(event: &str) -> String {
	event.replace(' ', "_")
}

fn active_variable(event: &str) -> String {
	format!("{}_active", event_word(event))
}

fn outer_variable(event: &str, number: usize) -> String {
	format!("{}_outer_{number}", event_word(event))
}

fn active_function(event: &str) -> String {
	generated(&[&event_word(event), "active"])
}

fn leave_function(event: &str) -> String {
	generated(&[&event_word(event), "leave"])
}

fn enter_name(event: &str, number: usize) -> String {
	generated(&[&event_word(event), "enter", &number.to_string()])
}

fn handler_name(event: &str, number: usize) -> String {
	generated(&[&event_word(event), "handler", &number.to_string()])
}
