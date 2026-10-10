//! Signals as values (notes/signals.md phase 5): a listener inside a function on one of its parameters, or on a
//! main-level variable the function does not assign, subscribes when the function runs and stays after it returns:
//! `watch(s) := on change s {print value}; x = 1; watch(x); x = 2` prints 2. Such a variable escapes into a $Signal, a
//! cell with a list of listener closures (wasm_emitter/cells.rs); every other variable stays plain (P109).
//! `subscribe` (before variable_signals) turns the listener into `signal_listeners_set(s, signal_listeners(s) +
//! [(value, old) => {check; 0}])`. `lower` (after it) finds the escaping variables (the subscribed parameters, and
//! through every call the variables passed for them) and makes their first main-level write `x = signal_new(e)`, every
//! other write `signal·set(x, e)` (store, then call each listener with the new and the old value) and every read
//! `cell_get(x)`. A function parameter that subscribes takes the signal itself: `watch(x)` passes x's cell.
//! Phase 6, `poll_shared` (before shared_arrays): a task writes a `shared` value in its own instance, so a main-level
//! listener watching one cannot be a check after the write. It becomes a check in the exported function `on·shared()`,
//! which the runtime calls at the program's check points (signal_poll at main's start and end and each loop start,
//! sleep, the end of the run): the watched values are compared with those seen last, and the listener runs on a change.

use super::words::{COUNT_WORD, FOR_WORD, FROM_WORD, GLOBAL_WORD, IN_WORD, OF_WORD};
use super::nodes::{call, children_rewritten, key};
use crate::declarations::word;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::variable_signals::{assign, block, defines_function, if_then, is_call_head, listener_parts, symbols, with_old, with_value, ListenerWord};
use crate::wasm_emitter::cells::{CELL_GET, CELL_NEW, CELL_SET, SIGNAL_LISTENERS, SIGNAL_LISTENERS_SET, SIGNAL_NEW};
use std::collections::{HashMap, HashSet};

/// `signal·set(s, v)`: store v in the signal s, then call its listeners
const SET_FUNCTION: &str = "signal·set";
/// The parameters of a listener closure: the new and the old value
const VALUE_WORD: &str = "value";
const OLD_WORD: &str = "signal·old";
const LISTENER_WORD: &str = "signal·listener";
/// The names the templates are written with (the parser reads `·` as a product)
const SET_PLACEHOLDER: &str = "signal_set_placeholder";
const OLD_PLACEHOLDER: &str = "signal_old_placeholder";
const LISTENER_PLACEHOLDER: &str = "signal_listener_placeholder";
const CHECK_PLACEHOLDER: &str = "signal_check_placeholder";
const CLOSURE_CALL: &str = "closure_call_2";
/// `signal·fired·0`: the cell of a once listener inside a function, whether it ran
const FIRED_PREFIX: &str = "signal·fired·";
/// `signal·held·0`: the cell of a whenever listener inside a function, whether its condition held; `whenever_was` the
/// value before the write, in the listener
const HELD_PREFIX: &str = "signal·held·";
const WAS_WORD: &str = "whenever_was";
/// `listeners of x`, `clear listeners of x` (card g-3HmY)
const LISTENERS_WORD: &str = "listeners";
const CLEAR_WORD: &str = "clear";
/// `remove alarm from listeners of t` (P128)
const REMOVE_WORD: &str = "remove";
/// `alarm_index_t`: the place of the named listener alarm in t's listeners
const INDEX_JOINER: &str = "_index_";
const WITHOUT_PLACEHOLDER: &str = "signal_without_placeholder";
const WITHOUT_FUNCTION: &str = "signal·without";

/// A function definition: its name, parameter names and body
struct Definition {
	name: String,
	params: Vec<String>,
	body: Node,
}

/// `signal_seen_0`: the value of a watched variable when the shared listeners last looked (main-level variables of
/// on·shared are declared `global` from text, so no `·` in them)
const SEEN_PREFIX: &str = "signal_seen_";
/// `signal_polling_0`: whether the shared listener was declared yet
const POLLING_PREFIX: &str = "signal_polling_";
/// `signal_writes_0`: the writes of a shared value an `on set` listener ran for (shared_writes)
const WRITES_PREFIX: &str = "signal_writes_";

/// Main-level listeners watching a `shared` value become checks of the function `on·shared()` the runtime polls
pub fn poll_shared(program: Node) -> Node {
	// a system value (system_values.rs) changes outside the program too
	let shared: HashSet<String> = crate::shared_arrays::shared_names(&program).into_iter().chain(crate::system_values::marked_names(&program)).collect();
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	if shared.is_empty() || !crate::variable_signals::is_statement_list(&bracket, &separator) {
		return program;
	}
	let mut main_variables = crate::event_signals::main_level_variables(&statements);
	let counted = crate::shared_arrays::shared_names(&program);
	let (mut lowered, mut checks, mut flags, mut seen_count) = (vec![], vec![], vec![], 0);
	for statement in statements {
		let parts = listener_parts(&statement).filter(|(_, subject, _)| symbols(subject).iter().any(|name| shared.contains(name)));
		let Some((listener_word, subject, body)) = parts else {
			lowered.push(statement);
			continue;
		};
		let polling = format!("{POLLING_PREFIX}{}", flags.len());
		// `on set n` of a shared value runs once per write, also for writes a task made between two polls
		let counted_name = match (listener_word, subject.drop_meta()) {
			(ListenerWord::Set, Node::Symbol(name)) if counted.contains(name) => Some(name.clone()),
			_ => None,
		};
		if let Some(name) = counted_name {
			let (writes, running) = (format!("{WRITES_PREFIX}{}", flags.len()), format!("{POLLING_PREFIX}{}_running", flags.len()));
			// the loop's start polls again: the running flag keeps that inner check out
			let each_write = format!("if {polling} and not {running} {{ {running} = true; while {writes} < {}({name}) {{ {writes} += 1; BODY }}; {running} = false }}", crate::host::SHARED_WRITES);
			let body = with_value(&body, &subject);
			checks.push(crate::law::substitute(crate::warp_parser::parse(&each_write).drop_meta(), &HashMap::from([("BODY".to_string(), body)])));
			lowered.push(crate::warp_parser::parse(&format!("{writes} = {}({name})", crate::host::SHARED_WRITES)).drop_meta().clone());
			lowered.push(assign(&polling, Node::True));
			flags.extend([polling, running]);
			main_variables.extend(flags.iter().cloned().chain([writes]));
			continue;
		}
		let watched = unique(symbols(&subject).into_iter().filter(|name| shared.contains(name) || main_variables.contains(name)).collect());
		let seen: Vec<(String, String)> = watched.into_iter().map(|name| { seen_count += 1; (name, format!("{SEEN_PREFIX}{}", seen_count - 1)) }).collect();
		let changed = seen.iter().map(|(name, last)| key(Node::Symbol(name.clone()), Op::Ne, Node::Symbol(last.clone())))
			.reduce(|either, next| key(either, Op::Or, next)).expect("a watched shared value");
		let value = seen.first().map(|(name, _)| Node::Symbol(name.clone())).expect("a watched value");
		let check = match listener_word {
			ListenerWord::Set | ListenerWord::Change => with_value(&body, &value),
			ListenerWord::Whenever => {
				let (held, was) = (format!("{POLLING_PREFIX}{}_held", flags.len()), format!("{POLLING_PREFIX}{}_was", flags.len()));
				flags.extend([held.clone(), was.clone()]);
				crate::variable_signals::edge_check(&held, &was, subject, body)
			}
			ListenerWord::Once => {
				let fired = format!("{POLLING_PREFIX}{}_fired", flags.len());
				flags.push(fired.clone());
				let condition = key(key(Node::Empty, Op::Not, Node::Symbol(fired.clone())), Op::And, subject);
				if_then(condition, block(vec![assign(&fired, Node::True), body]))
			}
		};
		let remember = seen.iter().map(|(name, last)| assign(last, Node::Symbol(name.clone())));
		checks.push(if_then(key(Node::Symbol(polling.clone()), Op::And, changed), block(remember.clone().chain([check]).collect())));
		lowered.extend(remember);
		lowered.push(assign(&polling, Node::True));
		flags.push(polling);
		main_variables.extend(flags.iter().cloned().chain(seen.into_iter().map(|(_, last)| last)));
	}
	if checks.is_empty() {
		return program;
	}
	// a shared value is no global of the instance: shared_arrays makes its uses host words
	let globals: HashSet<String> = main_variables.difference(&shared).cloned().collect();
	let handler = crate::event_signals::function_with_globals(crate::host::SHARED_HANDLER, false, &checks, &globals);
	let starts = flags.iter().map(|flag| assign(flag, Node::False));
	Node::List(starts.chain(std::iter::once(handler)).chain(lowered).collect(), bracket, separator)
}

/// Listeners inside functions on their parameters or on main-level variables become subscriptions made at run time
pub fn subscribe(program: Node) -> Node {
	let (program, mut reflected) = reflections(with_function_listeners(program));
	let main_variables = main_variables(&program);
	let mut subscriptions = Subscriptions { main_variables, fired: 0 };
	let program = subscriptions.rewrite(program);
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	if !crate::variable_signals::is_statement_list(&bracket, &separator) {
		return program;
	}
	// P128: a named listener `alarm = whenever t > 30 {…}` can be removed, so its variables are reflected too
	let named: Vec<(String, String)> = statements.iter().filter_map(named_listener).flat_map(|(name, listener)| {
		let watched = listener_parts(&listener).map(|(listener_word, subject, _)| watched_names(listener_word, &subject)).unwrap_or_default();
		watched.into_iter().map(move |variable| (name.clone(), variable))
	}).collect();
	reflected.extend(named.iter().map(|(_, variable)| variable.clone()));
	if reflected.is_empty() {
		return program;
	}
	// a reflected variable's main-level listeners subscribe too, so its list holds them
	let removes = statements.iter().any(contains_removal);
	let statements = statements.into_iter().flat_map(|statement| {
		if let Some((name, variable)) = removal(&statement) {
			return vec![removing(&name, &variable, &named)];
		}
		let subscribed = match named_listener(&statement) {
			Some((name, listener)) => subscriptions.subscription(&listener, &reflected, Some(&name)),
			None => subscriptions.subscription(&statement, &reflected, None),
		};
		// the statements of a subscription stay main-level ones: a named listener's variables are the program's
		match subscribed {
			Some(Node::List(parts, Bracket::Curly, _)) => parts,
			Some(subscription) => vec![subscription],
			None => vec![nested_removals(statement, &named)],
		}
	});
	let without = removes.then(without_function);
	Node::List(without.into_iter().chain(statements).collect(), bracket, separator)
}

/// `alarm = on change t {…}`, `alarm = whenever t > 30 {…}` (which arrives as `(alarm = (whenever t) > 30) {…}`): the
/// name and the listener
fn named_listener(statement: &Node) -> Option<(String, Node)> {
	if let Node::Key(name, Op::Assign, listener) = statement.drop_meta() {
		let Node::Symbol(name) = name.drop_meta() else { return None };
		return listener_parts(listener).map(|_| (name.clone(), listener.as_ref().clone()));
	}
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [head, body] = items.as_slice() else { return None };
	let Node::Key(name, Op::Assign, condition) = head.drop_meta() else { return None };
	let Node::Symbol(name) = name.drop_meta() else { return None };
	let (keyword, condition) = without_leading_word(condition)?;
	let listener = Node::List(vec![keyword, condition, body.clone()], Bracket::None, Separator::Space);
	listener_parts(&listener).map(|_| (name.clone(), listener))
}

/// `watch(s) := whenever s > 5 {…}` arrives as `(watch(s) := (whenever s) > 5) {…}`, its block a statement of its own:
/// the function `watch(s) := { whenever s > 5 {…} }` (card whenever-without)
fn with_function_listeners(node: Node) -> Node {
	let node = node.map_children(with_function_listeners);
	let Node::List(items, Bracket::None, _) = node.drop_meta() else { return node };
	let [head, body] = items.as_slice() else { return node };
	let Node::Key(function, Op::Define, value) = head.drop_meta() else { return node };
	let Some((keyword, condition)) = without_leading_word(value) else { return node };
	let listener = Node::List(vec![keyword, condition, body.clone()], Bracket::None, Separator::Space);
	if listener_parts(&listener).is_none() {
		return node;
	}
	Node::Key(function.clone(), Op::Define, Box::new(Node::List(vec![listener], Bracket::Curly, Separator::Semicolon)))
}

/// `(whenever t) > 30` → `whenever`, `t > 30`: the word the leftmost operand starts with, and the rest
fn without_leading_word(node: &Node) -> Option<(Node, Node)> {
	match node.drop_meta() {
		Node::Key(left, op, right) => without_leading_word(left).map(|(keyword, left)| (keyword, Node::Key(Box::new(left), *op, right.clone()))),
		Node::List(items, Bracket::None, separator) if items.len() >= 2 => {
			let rest = if items.len() == 2 { items[1].clone() } else { Node::List(items[1..].to_vec(), Bracket::None, separator.clone()) };
			Some((items[0].clone(), rest))
		}
		_ => None,
	}
}

/// The variables a listener watches: `on set x` and `on change x` the one, `whenever` and `once` those of the condition
fn watched_names(listener_word: ListenerWord, subject: &Node) -> Vec<String> {
	match (listener_word, subject.drop_meta()) {
		(ListenerWord::Set | ListenerWord::Change, Node::Symbol(name)) => vec![name.clone()],
		(ListenerWord::Set | ListenerWord::Change, _) => vec![],
		_ => unique(symbols(subject)),
	}
}

/// `remove alarm from listeners of t` (listeners already `signal_listeners(t)`): the listener's name and the variable
fn removal(statement: &Node) -> Option<(String, String)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [remove, name, from, list] = items.as_slice() else { return None };
	let Node::List(call_items, _, _) = list.drop_meta() else { return None };
	let [listeners, variable] = call_items.as_slice() else { return None };
	let reads = word(remove) == REMOVE_WORD && word(from) == FROM_WORD && word(listeners) == SIGNAL_LISTENERS;
	reads.then(|| (word(name), word(variable)))
}

fn contains_removal(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= removal(part).is_some());
	found
}

/// A removal inside a function or a block (`stop() := { remove b from listeners of t }`): the places it changes are
/// main-level variables, declared global where it runs
fn nested_removals(node: Node, named: &[(String, String)]) -> Node {
	if let Some((name, variable)) = removal(&node) {
		let globals = named.iter().filter(|(_, watched)| *watched == variable).map(|(other, _)| from_template(&format!("{GLOBAL_WORD} {}", index_name(other, &variable)), &[]));
		return block(globals.chain([removing(&name, &variable, named)]).collect());
	}
	children_rewritten(node, |child| nested_removals(child, named))
}

/// `alarm_index_t`: where the named listener alarm sits in t's listeners, -1 once removed
fn index_name(name: &str, variable: &str) -> String {
	format!("{name}{INDEX_JOINER}{variable}")
}

/// The listener leaves t's list; the named listeners after it move one place up
fn removing(name: &str, variable: &str, named: &[(String, String)]) -> Node {
	let index = index_name(name, variable);
	let shifts: String = named.iter().filter(|(other, watched)| other != name && watched == variable)
		.map(|(other, _)| { let other = index_name(other, variable); format!("if {other} > {index} {{ {other} = {other} - 1 }}; ") }).collect();
	from_template(&format!("if {index} >= 0 {{ {SIGNAL_LISTENERS_SET}({variable}, {WITHOUT_PLACEHOLDER}({variable}, {index})); {shifts}{index} = -1 }}"), &[])
}

/// `signal·without(s, skip)`: the listeners of s but the one at skip
fn without_function() -> Node {
	from_template(&format!("{WITHOUT_PLACEHOLDER}(s, skip) := {{ kept = []; i = 0; for f in {SIGNAL_LISTENERS}(s) {{ if i != skip {{ kept = kept + [f] }}; i += 1 }}; kept }}"), &[])
}

/// `listeners of x` → `signal_listeners(x)`, `clear listeners of x` → `signal_listeners_set(x, ø)`; and the variables
/// reflected so
fn reflections(program: Node) -> (Node, HashSet<String>) {
	let mut reflected = HashSet::new();
	let program = reflected_lists(program, &mut reflected);
	(program, reflected)
}

/// `for f in listeners of x {…}` arrives grouped `(listeners of) (x {…})`: its parts in one row
pub(crate) fn ungrouped_reflection(items: Vec<Node>) -> Vec<Node> {
	let grouped_reflection = |item: &Node| matches!(item.drop_meta(), Node::List(parts, Bracket::None, _) if parts.last().is_some_and(|last| word(last) == OF_WORD) && parts.iter().any(|part| word(part) == LISTENERS_WORD));
	if !items.iter().any(grouped_reflection) {
		return items;
	}
	items.into_iter().flat_map(|item| match item.drop_meta() {
		Node::List(parts, Bracket::None, _) => parts.clone(),
		_ => vec![item],
	}).collect()
}

fn reflected_lists(node: Node, reflected: &mut HashSet<String>) -> Node {
	let Node::List(items, bracket, separator) = node else { return children_rewritten(node, |child| reflected_lists(child, reflected)) };
	let mut out: Vec<Node> = vec![];
	let items: Vec<Node> = ungrouped_reflection(items).into_iter().map(|item| reflected_lists(item, reflected)).collect();
	let mut rest = items.into_iter().peekable();
	while let Some(item) = rest.next() {
		if word(&item) == LISTENERS_WORD && rest.peek().is_some_and(|next| word(next) == OF_WORD) {
			rest.next();
			// C#'s `listeners of x -= alarm` is `remove alarm from listeners of x`
			if let Some(Node::Key(variable, Op::SubAssign, name)) = rest.peek().map(|next| next.drop_meta().clone()) {
				rest.next();
				reflected.insert(word(&variable));
				out.extend([symbol(REMOVE_WORD), *name, symbol(FROM_WORD), call(SIGNAL_LISTENERS, vec![*variable])]);
				continue;
			}
			if let Some(variable) = rest.next() {
				reflected.insert(word(&variable));
				let list = call(SIGNAL_LISTENERS, vec![variable.clone()]);
				match out.last().map(word) {
					Some(clear) if clear == CLEAR_WORD => {
						out.pop();
						out.push(call(SIGNAL_LISTENERS_SET, vec![variable, Node::Empty]));
					}
					_ => out.push(list),
				}
				continue;
			}
		}
		out.push(item);
	}
	match (out.len(), &bracket) {
		(1, Bracket::None) => out.remove(0),
		_ => Node::List(out, bracket, separator),
	}
}

struct Subscriptions {
	main_variables: HashSet<String>,
	/// once listeners so far: each gets its own flag
	fired: usize,
}

impl Subscriptions {
	fn rewrite(&mut self, node: Node) -> Node {
		if let Some(definition) = definition(&node) {
			let locals = assigned_names(&definition.body);
			let over_params: Vec<String> = loop_variables(&definition.body).into_iter().filter(|(_, list)| definition.params.contains(list)).map(|(variable, _)| variable).collect();
			let subscribable = |name: &String| definition.params.contains(name) || over_params.contains(name) || (self.main_variables.contains(name) && !locals.contains(name));
			let subscribable: HashSet<String> = symbols(&definition.body).into_iter().filter(subscribable).collect();
			let body = self.subscriptions(definition.body.clone(), &subscribable);
			return with_body(node, body);
		}
		children_rewritten(node, |child| self.rewrite(child))
	}

	/// The listeners in a function body that watch a subscribable variable, as subscriptions
	fn subscriptions(&mut self, node: Node, subscribable: &HashSet<String>) -> Node {
		if definition(&node).is_some() {
			return self.rewrite(node);
		}
		if let Some(subscription) = self.subscription(&node, subscribable, None) {
			return subscription;
		}
		children_rewritten(node, |child| self.subscriptions(child, subscribable))
	}

	/// `on change s {body}` → `signal_listeners_set(s, signal_listeners(s) + [(value, signal·old) => {if value != signal·old {body}; 0}])`,
	/// one per watched variable; a once listener first makes its flag cell
	fn subscription(&mut self, statement: &Node, subscribable: &HashSet<String>, name: Option<&str>) -> Option<Node> {
		let (listener_word, subject, body) = listener_parts(statement)?;
		let watched: Vec<String> = watched_names(listener_word, &subject).into_iter().filter(|name| subscribable.contains(name)).collect();
		if watched.is_empty() {
			return None;
		}
		let value = symbol(VALUE_WORD);
		let mut statements = vec![];
		// `old` in `on set` / `on change` reads the listener closure's old value
		let body = match (listener_word, with_old(&body, &self.main_variables)) {
			(ListenerWord::Set | ListenerWord::Change, Some(with_old)) => with_old(&symbol(OLD_WORD)),
			_ => body,
		};
		let check = match listener_word {
			ListenerWord::Set => with_value(&body, &value),
			ListenerWord::Change => {
				if_then(key(value.clone(), Op::Ne, symbol(OLD_WORD)), block(vec![with_value(&body, &value)]))
			}
			// P156: when the condition becomes true; the cell holds whether it held at the last write
			ListenerWord::Whenever => {
				let held = format!("{HELD_PREFIX}{}", self.fired);
				self.fired += 1;
				statements.push(assign(&held, call(CELL_NEW, vec![Node::False])));
				let held_cell = Node::Symbol(held);
				// cells hold nodes: compared, not read as conditions
				let not_before = key(symbol(WAS_WORD), Op::Eq, Node::False);
				let holds = key(call(CELL_GET, vec![held_cell.clone()]), Op::Eq, Node::True);
				let became_true = key(holds, Op::And, not_before);
				let parts = vec![assign(WAS_WORD, call(CELL_GET, vec![held_cell.clone()])), call(CELL_SET, vec![held_cell, subject]), if_then(became_true, block(vec![body]))];
				Node::List(parts, Bracket::Round, Separator::Semicolon)
			}
			ListenerWord::Once => {
				let fired = format!("{FIRED_PREFIX}{}", self.fired);
				self.fired += 1;
				statements.push(assign(&fired, call(CELL_NEW, vec![Node::False])));
				let not_fired = key(call(CELL_GET, vec![Node::Symbol(fired.clone())]), Op::Eq, Node::False);
				let condition = key(not_fired, Op::And, subject);
				if_then(condition, block(vec![call(CELL_SET, vec![Node::Symbol(fired), Node::True]), body]))
			}
		};
		// the listener shares the main-level variables it changes, as a named one does (P124)
		let globals = crate::event_signals::global_declarations(std::slice::from_ref(&check), &self.main_variables, &[VALUE_WORD, OLD_WORD]);
		let closure_check = if globals.is_empty() { check.clone() } else { block(globals.iter().cloned().chain([check.clone()]).collect()) };
		// every listener closure of one arity returns one kind (wasm_emitter/closures.rs): 0
		let listener = from_template(&format!("({VALUE_WORD}, {OLD_PLACEHOLDER}) => {{ {CHECK_PLACEHOLDER}; 0 }}"), &[(CHECK_PLACEHOLDER, closure_check)]);
		// a named listener (P128) is a function of that name, and remembers where it sits in each list
		let listener = match name {
			Some(name) => {
				let head = call(name, vec![symbol(VALUE_WORD), symbol(OLD_WORD)]);
				let body = globals.into_iter().chain([check, crate::node::int(0)]).collect();
				statements.push(key(head, Op::Define, block(body)));
				for variable in &watched {
					let place = call(COUNT_WORD, vec![call(SIGNAL_LISTENERS, vec![Node::Symbol(variable.clone())])]);
					statements.push(assign(&index_name(name, variable), place));
				}
				symbol(name)
			}
			None => listener,
		};
		for name in watched {
			let signal = Node::Symbol(name);
			let listeners = key(call(SIGNAL_LISTENERS, vec![signal.clone()]), Op::Add, Node::List(vec![listener.clone()], Bracket::Square, Separator::None));
			statements.push(call(SIGNAL_LISTENERS_SET, vec![signal, listeners]));
		}
		Some(if statements.len() == 1 { statements.remove(0) } else { block(statements) })
	}
}

/// The escaping variables become signals: made at their first main-level write, set and read through their cell
pub fn lower(program: Node) -> Node {
	let mut definitions = vec![];
	collect_definitions(&program, &mut definitions);
	let Escaping { signal_params, list_params, signal_lists, escaping } = escaping(&program, &definitions);
	if signal_params.is_empty() && escaping.is_empty() {
		return program;
	}
	let mut signals = Signals { signal_params, list_params, signal_lists, escaping, made: HashSet::new() };
	let program = signals.rewrite(program, &signals.escaping.clone(), true);
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	Node::List(std::iter::once(set_function()).chain(statements).collect(), bracket, separator)
}

struct Signals {
	/// (function, parameter index) of each parameter that takes a signal
	signal_params: HashSet<(String, usize)>,
	/// (function, parameter index) of each parameter that takes a list of signals (`for s in xs {on change s …}`)
	list_params: HashSet<(String, usize)>,
	/// the main-level variables holding a list of signals: their list literals keep the signals themselves
	signal_lists: HashSet<String>,
	/// the main-level variables that are signals
	escaping: HashSet<String>,
	/// the escaping variables made so far, in the order of the main-level statements
	made: HashSet<String>,
}

impl Signals {
	fn rewrite(&mut self, node: Node, signals: &HashSet<String>, main: bool) -> Node {
		if let Some(definition) = definition(&node) {
			let locals = assigned_names(&definition.body);
			let globals = declared_globals(&definition.body);
			let mut inner: HashSet<String> = self.escaping.iter().filter(|name| !definition.params.contains(name) && (!locals.contains(*name) || globals.contains(*name))).cloned().collect();
			inner.extend(definition.params.iter().enumerate().filter(|(index, _)| self.signal_params.contains(&(definition.name.clone(), *index))).map(|(_, name)| name.clone()));
			let lists: Vec<&String> = definition.params.iter().enumerate().filter(|(index, _)| self.list_params.contains(&(definition.name.clone(), *index))).map(|(_, name)| name).collect();
			inner.extend(loop_variables(&definition.body).into_iter().filter(|(_, list)| lists.contains(&list)).map(|(variable, _)| variable));
			let body = self.rewrite(definition.body.clone(), &inner, false);
			return with_body(node, body);
		}
		match node {
			Node::Symbol(ref name) if signals.contains(name) => call(CELL_GET, vec![node]),
			// `for s in xs {…}` over signals: s names the signal, its uses read it
			Node::List(items, bracket, separator) if loop_variable(&items).is_some_and(|variable| signals.contains(&variable)) => {
				let items = items.into_iter().enumerate().map(|(index, item)| if index == 1 { item } else { self.rewrite(item, signals, main) }).collect();
				Node::List(items, bracket, separator)
			}
			// `watched = [a, b]` of a list of signals keeps the signals
			Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(name) if self.signal_lists.contains(name)) => {
				Node::Key(target, Op::Assign, Box::new(self.signals_kept(*value, signals, main)))
			}
			Node::Key(target, op, value) if is_write(op) && matches!(target.drop_meta(), Node::Symbol(name) if signals.contains(name)) => {
				let name = target.drop_meta().name();
				let value = match op {
					Op::Assign => self.rewrite(*value, signals, main),
					Op::Inc | Op::Dec => key(call(CELL_GET, vec![*target.clone()]), if op == Op::Inc { Op::Add } else { Op::Sub }, crate::node::int(1)),
					_ => key(call(CELL_GET, vec![*target.clone()]), op.base_op(), self.rewrite(*value, signals, main)),
				};
				if main && self.escaping.contains(&name) && self.made.insert(name) {
					return Node::Key(target, Op::Assign, Box::new(call(SIGNAL_NEW, vec![value])));
				}
				call(SET_FUNCTION, vec![*target, value])
			}
			Node::Key(keyword, Op::Colon, name) if word(&keyword) == GLOBAL_WORD => Node::Key(keyword, Op::Colon, name),
			// a function changes an escaping variable's cell, never the variable: its `global x` goes
			Node::List(items, bracket, separator) if items.iter().any(|item| declares_global_of(item, signals)) => {
				let kept = items.into_iter().filter(|item| !declares_global_of(item, signals));
				Node::List(kept.map(|item| self.rewrite(item, signals, main)).collect(), bracket, separator)
			}
			Node::Key(left, op @ (Op::Dot | Op::Colon), right) if !matches!(op, Op::Dot) || matches!(right.drop_meta(), Node::Symbol(_)) => {
				Node::Key(Box::new(self.rewrite(*left, signals, main)), op, right)
			}
			Node::List(items, bracket, separator) if self.is_signal_call(&items, signals) => {
				let function = word(&items[0]);
				let arguments = items.into_iter().enumerate().map(|(index, item)| match index {
					0 => item,
					_ if function == SIGNAL_LISTENERS_SET || function == SIGNAL_LISTENERS => {
						if index == 1 { item } else { self.rewrite(item, signals, main) }
					}
					_ if self.list_params.contains(&(function.clone(), index - 1)) => match item.drop_meta() {
						Node::Symbol(name) if self.signal_lists.contains(name) || signals.contains(name) => item,
						_ => self.signals_kept(item, signals, main),
					},
					_ if self.signal_params.contains(&(function.clone(), index - 1)) => match item.drop_meta() {
						Node::Symbol(name) if signals.contains(name) => item,
						_ => call(SIGNAL_NEW, vec![self.rewrite(item, signals, main)]),
					},
					_ => self.rewrite(item, signals, main),
				}).collect();
				Node::List(arguments, bracket, separator)
			}
			other => children_rewritten(other, |child| self.rewrite(child, signals, main)),
		}
	}

	/// A call that passes signals as they are: a subscription, or a function taking a signal parameter
	fn is_signal_call(&self, items: &[Node], signals: &HashSet<String>) -> bool {
		let Some(first) = items.first() else { return false };
		let function = word(first);
		let subscribes = (function == SIGNAL_LISTENERS_SET || function == SIGNAL_LISTENERS) && items.get(1).is_some_and(|signal| signals.contains(&word(signal)));
		let takes = |params: &HashSet<(String, usize)>| params.iter().any(|(name, _)| *name == function);
		subscribes || (items.len() > 1 && (takes(&self.signal_params) || takes(&self.list_params)))
	}

	/// A list literal of signals: its signal names stay the signals, everything else is rewritten as usual
	fn signals_kept(&mut self, list: Node, signals: &HashSet<String>, main: bool) -> Node {
		match list.drop_meta().clone() {
			Node::List(items, Bracket::Square, separator) => {
				let items = items.into_iter().map(|item| match item.drop_meta() {
					Node::Symbol(name) if signals.contains(name) => item,
					_ => self.rewrite(item, signals, main),
				}).collect();
				Node::List(items, Bracket::Square, separator)
			}
			other => self.rewrite(other, signals, main),
		}
	}
}

/// What escapes into signals
struct Escaping {
	signal_params: HashSet<(String, usize)>,
	list_params: HashSet<(String, usize)>,
	signal_lists: HashSet<String>,
	escaping: HashSet<String>,
}

/// The parameters that take a signal or a list of them, and the main-level variables that escape: subscribed to inside
/// a function, passed for such a parameter (through further calls too), or kept in a list passed for one
fn escaping(program: &Node, definitions: &[Definition]) -> Escaping {
	let mut found = Escaping { signal_params: HashSet::new(), list_params: HashSet::new(), signal_lists: HashSet::new(), escaping: HashSet::new() };
	// the main level subscribes or reflects (`listeners of x`) too
	without_definitions(program).visit(&mut |node| {
		let Node::List(items, _, _) = node else { return };
		if items.len() >= 2 && [SIGNAL_LISTENERS, SIGNAL_LISTENERS_SET].contains(&word(&items[0]).as_str()) {
			found.escaping.insert(word(&items[1]));
		}
	});
	for definition in definitions {
		let loops = loop_variables(&definition.body);
		definition.body.visit(&mut |node| {
			let Node::List(items, _, _) = node else { return };
			if (items.len() == 3 && word(&items[0]) == SIGNAL_LISTENERS_SET) || (items.len() == 2 && word(&items[0]) == SIGNAL_LISTENERS) {
				let name = word(&items[1]);
				let over = loops.iter().find(|(variable, _)| *variable == name).map(|(_, list)| list.clone());
				match (definition.params.iter().position(|param| *param == name), over.and_then(|list| definition.params.iter().position(|param| *param == list))) {
					(Some(index), _) => { found.signal_params.insert((definition.name.clone(), index)); }
					(None, Some(index)) => { found.list_params.insert((definition.name.clone(), index)); }
					(None, None) => { found.escaping.insert(name); }
				}
			}
		});
	}
	let main_lists = main_list_literals(program);
	loop {
		let before = (found.signal_params.len(), found.list_params.len(), found.signal_lists.len(), found.escaping.len());
		let mut passed = |scope: Option<&Definition>, node: &Node| {
			let Node::List(items, _, _) = node else { return };
			let Some(function) = items.first().map(word) else { return };
			let param_of_scope = |name: &str| scope.and_then(|definition| definition.params.iter().position(|param| param == name).map(|position| (definition.name.clone(), position)));
			for (index, argument) in items.iter().skip(1).enumerate() {
				let key = (function.clone(), index);
				if found.signal_params.contains(&key) {
					if let Node::Symbol(name) = argument.drop_meta() {
						match param_of_scope(name) {
							Some(param) => { found.signal_params.insert(param); }
							None => { found.escaping.insert(name.clone()); }
						}
					}
				}
				if found.list_params.contains(&key) {
					let names: Vec<String> = match argument.drop_meta() {
						Node::Symbol(name) => match param_of_scope(name) {
							Some(param) => {
								found.list_params.insert(param);
								vec![]
							}
							None => {
								found.signal_lists.insert(name.clone());
								main_lists.iter().filter(|(list, _)| list == name).flat_map(|(_, literal)| named_items(literal)).collect()
							}
						},
						literal => named_items(literal),
					};
					for name in names {
						match param_of_scope(&name) {
							Some(param) => { found.signal_params.insert(param); }
							None => { found.escaping.insert(name); }
						}
					}
				}
			}
		};
		without_definitions(program).visit(&mut |node| passed(None, node));
		for definition in definitions {
			definition.body.visit(&mut |node| passed(Some(definition), node));
		}
		if (found.signal_params.len(), found.list_params.len(), found.signal_lists.len(), found.escaping.len()) == before {
			return found;
		}
	}
}

/// The variables a list literal names as its items: `[a, b]` → a, b
fn named_items(list: &Node) -> Vec<String> {
	match list.drop_meta() {
		Node::List(items, Bracket::Square, _) => items.iter().filter_map(|item| match item.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		}).collect(),
		_ => vec![],
	}
}

/// The main-level `xs = [a, b]` assignments: each variable and its list literal
fn main_list_literals(program: &Node) -> Vec<(String, Node)> {
	let mut lists = vec![];
	without_definitions(program).visit(&mut |node| if let Node::Key(target, Op::Assign, value) = node {
		if let (Node::Symbol(name), Node::List(_, Bracket::Square, _)) = (target.drop_meta(), value.drop_meta()) {
			lists.push((name.clone(), value.as_ref().clone()));
		}
	});
	lists
}

/// `for s in xs {…}`: the loop variable s
fn loop_variable(items: &[Node]) -> Option<String> {
	match items {
		[keyword, variable, in_word, _, _] if word(keyword) == FOR_WORD && word(in_word) == IN_WORD => match variable.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// Each `for s in xs {…}` in the body: the loop variable and the name of the list
fn loop_variables(body: &Node) -> Vec<(String, String)> {
	let mut loops = vec![];
	body.visit(&mut |node| if let Node::List(items, _, _) = node {
		if let (Some(variable), Some(Node::Symbol(list))) = (loop_variable(items), items.get(3).map(Node::drop_meta)) {
			loops.push((variable, list.clone()));
		}
	});
	loops
}

/// `signal·set(s, v)`: store v, then call each listener with the new and the old value; gives the new value
fn set_function() -> Node {
	from_template(&format!("{SET_PLACEHOLDER}(s, v) := {{ {OLD_PLACEHOLDER} = {CELL_GET}(s); {CELL_SET}(s, v); for {LISTENER_PLACEHOLDER} in {SIGNAL_LISTENERS}(s) {{ {CLOSURE_CALL}({LISTENER_PLACEHOLDER}, {CELL_GET}(s), {OLD_PLACEHOLDER}) }}; {CELL_GET}(s) }}"), &[])
}

/// The code parsed, its placeholders replaced: the generated names (`signal·old`) and the given nodes
fn from_template(code: &str, nodes: &[(&str, Node)]) -> Node {
	let names = [(SET_PLACEHOLDER, SET_FUNCTION), (OLD_PLACEHOLDER, OLD_WORD), (LISTENER_PLACEHOLDER, LISTENER_WORD), (WITHOUT_PLACEHOLDER, WITHOUT_FUNCTION)];
	let bindings = names.iter().map(|(placeholder, name)| (placeholder.to_string(), symbol(name)))
		.chain(nodes.iter().map(|(placeholder, node)| (placeholder.to_string(), node.clone()))).collect();
	crate::law::substitute(&crate::warp_parser::parse(code), &bindings).drop_meta().clone()
}

fn is_write(op: Op) -> bool {
	op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec)
}

/// `f(a, b) := body`, `def f(a, b) {body}`
fn definition(node: &Node) -> Option<Definition> {
	let (head, body) = match node.drop_meta() {
		Node::Key(head, Op::Define, body) if is_call_head(head) => (head.as_ref().clone(), body.as_ref().clone()),
		Node::List(items, _, _) if defines_function(items) => {
			let parts = keyword_definition_parts(&items[1..]);
			let (head, body) = (parts.first()?.clone(), parts.last()?.clone());
			matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)).then_some(())?;
			(head, body)
		}
		_ => return None,
	};
	let Node::List(head_items, _, _) = head.drop_meta() else { return None };
	let name = word(head_items.first()?);
	let params = head_items[1..].iter().flat_map(parameter_names).collect();
	Some(Definition { name, params, body })
}

/// `def ((f (x)) {body})` arrives as one list after the keyword: its head and body
fn keyword_definition_parts(rest: &[Node]) -> Vec<Node> {
	match rest {
		[only] => match only.drop_meta() {
			Node::List(items, Bracket::None | Bracket::Round, _) if items.len() == 2 => items.clone(),
			_ => rest.to_vec(),
		},
		_ => rest.to_vec(),
	}
}

/// `x`, `(x, y)`, `ø`
fn parameter_names(param: &Node) -> Vec<String> {
	match param.drop_meta() {
		Node::Symbol(name) => vec![name.clone()],
		Node::List(items, _, _) => items.iter().flat_map(parameter_names).collect(),
		Node::Key(name, Op::Colon, _) => parameter_names(name),
		_ => vec![],
	}
}

/// The definition with another body (the last part of it)
fn with_body(node: Node, body: Node) -> Node {
	match node {
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_body(*node, body)), data },
		Node::Key(head, op, _) => Node::Key(head, op, Box::new(body)),
		Node::List(mut items, bracket, separator) => {
			let last = items.pop().expect("a definition has a body");
			items.push(match (last.drop_meta(), body.drop_meta()) {
				(Node::List(_, Bracket::Curly, _), Node::List(_, Bracket::Curly, _)) => body,
				(Node::List(_, Bracket::Curly, _), _) => block(vec![body]),
				_ => with_body(last, body),
			});
			Node::List(items, bracket, separator)
		}
		other => other,
	}
}

fn collect_definitions(node: &Node, definitions: &mut Vec<Definition>) {
	if let Some(definition) = definition(node) {
		collect_definitions(&definition.body, definitions);
		definitions.push(definition);
		return;
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => {
			collect_definitions(left, definitions);
			collect_definitions(right, definitions);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_definitions(item, definitions)),
		_ => {}
	}
}

/// The program without its function definitions: what runs at the main level
fn without_definitions(node: &Node) -> Node {
	if definition(node).is_some() {
		return Node::Empty;
	}
	children_rewritten(node.clone(), |child| without_definitions(&child))
}

/// The names the main level assigns
fn main_variables(program: &Node) -> HashSet<String> {
	match program.drop_meta() {
		Node::List(items, bracket, separator) if crate::variable_signals::is_statement_list(bracket, separator) => crate::event_signals::main_level_variables(items),
		statement => crate::event_signals::main_level_variables(std::slice::from_ref(statement)),
	}
}

/// The names a function body assigns (its locals, unless declared global), not those of the functions nested in it
fn assigned_names(body: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	without_definitions(body).visit(&mut |node| if let Node::Key(target, op, _) = node {
		if let (true, Node::Symbol(name)) = (is_write(*op), target.drop_meta()) {
			names.insert(name.clone());
		}
	});
	names
}

/// `global x` of one of the names
fn declares_global_of(statement: &Node, names: &HashSet<String>) -> bool {
	matches!(statement.drop_meta(), Node::Key(keyword, Op::Colon, name) if word(keyword) == GLOBAL_WORD && names.contains(&word(name)))
}

fn declared_globals(body: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	body.visit(&mut |node| if let Node::Key(keyword, Op::Colon, name) = node {
		if word(keyword) == GLOBAL_WORD {
			names.insert(word(name));
		}
	});
	names
}

fn unique(names: Vec<String>) -> Vec<String> {
	let mut seen = HashSet::new();
	names.into_iter().filter(|name| seen.insert(name.clone())).collect()
}
