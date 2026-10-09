//! Block-scoped event handlers (card effect-handlers step 1, notes/effect_handlers.md): `on ask { 42 } in { compute() }`
//! answers `emit ask` while the block runs, in it and in the functions it calls; the innermost handler wins and the
//! handler's value is what emit gives (tail-resumptive, plain calls). Before event_signals: an emit no block handler
//! takes stays the emit, which the program-wide `on ask {…}` answers as before.
//! Handler i of ask is the function `ask·handler·i(event)`; the global `effect_handler_active_ask` names the active one
//! (0: none) and `effect_handler_outer_ask_i` the one active when block i was entered, which an emit inside handler i
//! reaches.
//! `break value` in a handler body aborts (step 3): the handler stores value in `effect_handler_aborted_ask_i` and
//! throws to its block, which runs under `ran_without_abort(i, {…})` and then has that value.

use crate::event_signals::{emit_verbs, emitted, function_with_globals, main_level_variables, reads_event, statements_of};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::variable_signals::{assign, if_then_else};
use std::collections::BTreeMap;

const ON_WORD: &str = "on";
const IN_WORD: &str = "in";
/// Generated function names join their parts with it
const JOINER: &str = "·";
/// The variable the handler function runs its body into before it restores the active handler
const RESULT_WORD: &str = "handled";
const SAVED_WORD: &str = "saved";
/// The plumbing's global variables (`effect_handler_active_ask`): plain words, as `global` parses them
const PLUMBING_PREFIX: &str = "effect_handler_";
/// `break value` in a handler body: its block ends with value
const BREAK_WORD: &str = "break";
const FINISHED_WORD: &str = "finished";

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
		.chain(handlers.iter().map(|handler| assign(&outer_variable(&handler.event, handler.number), Node::int(0))))
		.chain(handlers.iter().filter(|handler| aborts(&handler.body)).map(|handler| assign(&aborted_variable(&handler.event, handler.number), Node::Empty)));
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

/// Does a block handler of the program answer the event (its dispatch function `ask·active` is defined)?
pub(crate) fn has_block_handler(program: &Node, event: &str) -> bool {
	let dispatch = active_function(event);
	crate::variable_signals::symbols(program).contains(&dispatch)
}

/// The program with each `on ask {…} in {…}` replaced by its block run under handler i, the handlers collected
fn scoped_blocks(node: Node, handlers: &mut Vec<Handler>) -> Node {
	let node = operand_phrase(node).map_children(|child| scoped_blocks(child, handlers));
	let Some((event, body, block)) = scoped_handler(&node) else { return node };
	let number = handlers.len() + 1;
	let saved = generated(&[SAVED_WORD, &event_word(&event), &number.to_string()]);
	let value = generated(&[RESULT_WORD, &event_word(&event), &number.to_string()]);
	let enter = assign(&saved, call(&enter_name(&event, number), vec![]));
	let run_block = assign(&value, sequence(statements_of(&block)));
	let leave = call(&leave_function(&event), vec![Node::Symbol(saved)]);
	let run = if aborts(&body) {
		// `finished = ran_without_abort(i, {value = block}); leave; if finished then value else aborted_i`
		let finished = generated(&[FINISHED_WORD, &event_word(&event), &number.to_string()]);
		let guarded = Node::List(vec![run_block], Bracket::Curly, Separator::Semicolon);
		let ran = call(crate::wasm_emitter::RAN_WITHOUT_ABORT, vec![Node::int(number as i64), guarded]);
		let value_or_aborted = if_then_else(Node::Symbol(finished.clone()), Node::Symbol(value), Node::Symbol(aborted_variable(&event, number)));
		sequence(vec![enter, assign(&finished, ran), leave, value_or_aborted])
	} else {
		sequence(vec![enter, run_block, leave, Node::Symbol(value)])
	};
	handlers.push(Handler { event: event.clone(), number, body });
	// `{ on ask {…} in {…} }`, a block of this one statement, stays a block
	match node.drop_meta() {
		Node::List(_, Bracket::Curly, _) => Node::List(vec![run], Bracket::Curly, Separator::Semicolon),
		_ => run,
	}
}

/// `on ask {body} in {block}`: the event, the handler body and the block
pub(crate) fn scoped_handler(node: &Node) -> Option<(String, Node, Node)> {
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

/// `print "x " + on ask {…} in {…}` parses as `print ("x " + on) ask {…} in {…}`: the handler phrase back in the place
/// of its `on`, a word of the phrase or the last operand of an operator (`"x " + (on ask {…} in {…})`)
fn operand_phrase(node: Node) -> Node {
	let Node::List(items, _, Separator::Space) = node.drop_meta() else { return node };
	let items = phrase_words(items);
	let starts_handler = |index: &usize| {
		let rest = [&[Node::Symbol(ON_WORD.into())], &items[index + 1..]].concat();
		scoped_handler(&Node::List(rest, Bracket::None, Separator::Space)).is_some()
	};
	let Some(start) = (0..items.len()).filter(|index| ends_with_on(&items[*index])).find(|index| *index > 0 || !is_on(&items[0])).filter(starts_handler) else { return node };
	let phrase = Node::List([&[Node::Symbol(ON_WORD.into())], &items[start + 1..]].concat(), Bracket::None, Separator::Space);
	let head = &items[..start];
	let placed = with_last_operand(items[start].clone(), phrase);
	match head.is_empty() {
		true => placed,
		false => Node::List([head, &[placed]].concat(), Bracket::None, Separator::Space),
	}
}

fn is_on(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(word) if word == ON_WORD)
}

fn ends_with_on(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, _, right) => ends_with_on(right),
		other => is_on(other),
	}
}

/// `a + on` → `a + phrase`, `on` → phrase
fn with_last_operand(node: Node, phrase: Node) -> Node {
	match node.drop_meta() {
		Node::Key(left, op, right) => Node::Key(left.clone(), *op, Box::new(with_last_operand(*right.clone(), phrase))),
		_ => phrase,
	}
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

/// `ask·enter·i() := { global active, outer_i; outer_i = active; active = i; outer_i }`: handler i active, the one
/// before it kept
fn enter_function(handler: &Handler, main_variables: &std::collections::HashSet<String>) -> Node {
	let (active, outer) = (active_variable(&handler.event), outer_variable(&handler.event, handler.number));
	let body = [assign(&outer, Node::Symbol(active.clone())), assign(&active, Node::int(handler.number as i64)), Node::Symbol(outer)];
	function_with_globals(&enter_name(&handler.event, handler.number), false, &body, main_variables)
}

/// `ask·handler·i(event) := { previous = active; active = outer_i; handled = (body); active = previous; handled }`:
/// while the body runs, an emit of ask reaches the handler outside block i
fn handler_function(handler: &Handler, by_event: &BTreeMap<String, Vec<usize>>, verbs: &[String], main_variables: &std::collections::HashSet<String>) -> Node {
	let active = active_variable(&handler.event);
	let previous = generated(&[SAVED_WORD, "previous"]);
	let result = generated(&[RESULT_WORD, "value"]);
	let statements = statements_of(&handler.body).into_iter().map(|statement| match broken_value(&statement) {
		// `break value`: the value for the block, then the jump to it
		Some(value) => sequence(vec![assign(&aborted_variable(&handler.event, handler.number), value), call(crate::wasm_emitter::ABORT_TO, vec![Node::int(handler.number as i64)])]),
		None => statement,
	}).collect();
	let body = dispatched_emits(sequence(statements), by_event, verbs);
	let statements = [
		assign(&previous, Node::Symbol(active.clone())),
		assign(&active, Node::Symbol(outer_variable(&handler.event, handler.number))),
		assign(&result, body),
		assign(&active, Node::Symbol(previous)),
		Node::Symbol(result),
	];
	function_with_globals(&handler_name(&handler.event, handler.number), reads_event(std::slice::from_ref(&handler.body)), &statements, main_variables)
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
	format!("{PLUMBING_PREFIX}active_{}", event_word(event))
}

fn outer_variable(event: &str, number: usize) -> String {
	format!("{PLUMBING_PREFIX}outer_{}_{number}", event_word(event))
}

/// A variable of the handler plumbing: no state of the program's own (effects.rs keeps a function using it pure)
pub(crate) fn is_plumbing_variable(name: &str) -> bool {
	name.starts_with(PLUMBING_PREFIX)
}

fn aborted_variable(event: &str, number: usize) -> String {
	format!("{PLUMBING_PREFIX}aborted_{}_{number}", event_word(event))
}

/// Does a handler body end its block: a `break` among its statements (one inside a loop belongs to the loop)
fn aborts(body: &Node) -> bool {
	statements_of(body).iter().any(|statement| broken_value(statement).is_some())
}

/// `break value` (ø for a bare `break`): the value its block ends with
fn broken_value(statement: &Node) -> Option<Node> {
	let is_break = |node: &Node| matches!(node.drop_meta(), Node::Symbol(word) if word == BREAK_WORD);
	match statement.drop_meta() {
		word if is_break(word) => Some(Node::Empty),
		// `break -1` parses as the subtraction `break - 1`
		Node::Key(word, Op::Sub, value) if is_break(word) => Some(Node::Key(Box::new(Node::int(0)), Op::Sub, value.clone())),
		Node::List(items, _, Separator::Space) if items.first().is_some_and(is_break) => Some(match &items[1..] {
			[value] => value.clone(),
			words => Node::List(words.to_vec(), Bracket::None, Separator::Space),
		}),
		_ => None,
	}
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
