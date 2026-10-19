//! Signals as values (notes/signals.md phase 5): a listener inside a function on one of its parameters, or on a
//! main-level variable the function does not assign, subscribes when the function runs and stays after it returns:
//! `watch(s) := on change s {print value}; x = 1; watch(x); x = 2` prints 2. Such a variable escapes into a $Signal, a
//! cell with a list of listener closures (wasm_emitter/functions/cells.rs); every other variable stays plain (P109).
//! `subscribe` (before variable_signals) turns the listener into `signal_listeners_set(s, signal_listeners(s) +
//! [(value, old) => {check; 0}])`. `lower` (after it) finds the escaping variables (the subscribed parameters, and
//! through every call the variables passed for them) and makes their first main-level write `x = signal_new(e)`, every
//! other write `signal·set(x, e)` (store, then call each listener with the new and the old value) and every read
//! `cell_get(x)`. A function parameter that subscribes takes the signal itself: `watch(x)` passes x's cell.
//! Phase 6, `poll_shared` (before shared_arrays): a task writes a `shared` value in its own instance, so a main-level
//! listener watching one cannot be a check after the write. It becomes a check in the exported function `on·shared()`,
//! which the runtime calls at the program's check points (signal_poll at main's start and end and each loop start,
//! sleep, the end of the run): the watched values are compared with those seen last, and the listener runs on a change.

use super::listeners;
use super::nodes::{any_part, assign, block, call, children_rewritten, for_in_variable, if_then, key};
use crate::declarations::word;
use crate::lowering::{function_head, DefinitionForm};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::variable_signals::{
	assign_variable, is_round_head, listener_parts, symbols, with_old, with_value, ListenerWord,
};
use crate::wasm_emitter::functions::cells::{
	CELL_GET, CELL_NEW, CELL_SET, SIGNAL_LISTENERS, SIGNAL_LISTENERS_SET, SIGNAL_NEW,
};
use crate::words::{COUNT_WORD, FROM_WORD, GLOBAL_WORD, ONCE_WORD, ON_WORD, WHENEVER_WORD};
use std::collections::HashSet;

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
/// A field's signal is named by its path, `p.age` (cards signal-param-field, signal-field-escape)
const FIELD_JOINER: char = '.';

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
	let shared: HashSet<String> = crate::shared_arrays::shared_names(&program)
		.into_iter()
		.chain(crate::system_values::marked_names(&program))
		.collect();
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else {
		return program;
	};
	if shared.is_empty() || !crate::variable_signals::is_statement_list(&bracket, &separator) {
		return program;
	}
	let mut polls = SharedPolls {
		main_variables: crate::event_signals::main_level_variables(&statements),
		counted: crate::shared_arrays::shared_names(&program),
		shared,
		lowered: vec![],
		checks: vec![],
		flags: vec![],
		seen_count: 0,
	};
	for statement in statements {
		polls.lower(statement);
	}
	if polls.checks.is_empty() {
		return program;
	}
	// a shared value is no global of the instance: shared_arrays makes its uses host words
	let globals: HashSet<String> = polls.main_variables.difference(&polls.shared).cloned().collect();
	let handler =
		crate::event_signals::function_with_globals(crate::host::SHARED_HANDLER, false, &polls.checks, &globals);
	let starts = polls.flags.iter().map(|flag| assign_variable(flag, Node::False));
	Node::List(starts.chain(std::iter::once(handler)).chain(polls.lowered).collect(), bracket, separator)
}

/// poll_shared's main-level statements, the checks of on·shared and the flags and seen values they keep
struct SharedPolls {
	shared: HashSet<String>,
	/// the shared values whose writes are counted (shared_writes)
	counted: Vec<String>,
	main_variables: HashSet<String>,
	lowered: Vec<Node>,
	checks: Vec<Node>,
	flags: Vec<String>,
	seen_count: usize,
}

impl SharedPolls {
	fn lower(&mut self, statement: Node) {
		let parts = listener_parts(&statement)
			.filter(|(_, subject, _)| symbols(subject).iter().any(|name| self.shared.contains(name)));
		let Some((listener_word, subject, body)) = parts else {
			self.lowered.push(statement);
			return;
		};
		let polling = format!("{POLLING_PREFIX}{}", self.flags.len());
		// `on set n` of a shared value runs once per write, also for writes a task made between two polls
		match (listener_word, subject.drop_meta()) {
			(ListenerWord::Set, Node::Symbol(name)) if self.counted.contains(name) => {
				let name = name.clone();
				self.each_write(polling, &name, &subject, &body)
			}
			_ => self.on_change(polling, listener_word, subject, body),
		}
	}

	/// `on set n` of a counted shared value: the body once per write since the last poll
	fn each_write(&mut self, polling: String, name: &str, subject: &Node, body: &Node) {
		let (writes, running) =
			(format!("{WRITES_PREFIX}{}", self.flags.len()), format!("{POLLING_PREFIX}{}_running", self.flags.len()));
		// the loop's start polls again: the running flag keeps that inner check out
		let each_write = format!(
			"if {polling} and not {running} {{ {running} = true; while {writes} < {}({name}) {{ {writes} += 1; BODY }}; {running} = false }}",
			crate::host::SHARED_WRITES
		);
		let body = with_value(body, subject);
		self.checks.push(super::nodes::filled(&each_write, [("BODY", body)]));
		self.lowered.push(
			crate::warp_parser::parse_generated(&format!("{writes} = {}({name})", crate::host::SHARED_WRITES))
				.drop_meta()
				.clone(),
		);
		self.lowered.push(assign_variable(&polling, Node::True));
		self.flags.extend([polling, running]);
		self.main_variables.extend(self.flags.iter().cloned().chain([writes]));
	}

	/// A listener on watched values: it runs when one differs from the value seen last
	fn on_change(&mut self, polling: String, listener_word: ListenerWord, subject: Node, body: Node) {
		let seen: Vec<(String, String)> = subject
			.symbols_read()
			.into_iter()
			.filter(|name| self.shared.contains(name) || self.main_variables.contains(name))
			.map(|name| {
				self.seen_count += 1;
				(name, format!("{SEEN_PREFIX}{}", self.seen_count - 1))
			})
			.collect();
		let changed = seen
			.iter()
			.map(|(name, last)| key(Node::Symbol(name.clone()), Op::Ne, Node::Symbol(last.clone())))
			.reduce(|either, next| key(either, Op::Or, next))
			.expect("a watched shared value");
		let value = seen.first().map(|(name, _)| Node::Symbol(name.clone())).expect("a watched value");
		let check = self.check(listener_word, subject, body, &value);
		let remember = seen.iter().map(|(name, last)| assign_variable(last, Node::Symbol(name.clone())));
		self.checks.push(if_then(
			key(Node::Symbol(polling.clone()), Op::And, changed),
			block(remember.clone().chain([check]).collect()),
		));
		self.lowered.extend(remember);
		self.lowered.push(assign_variable(&polling, Node::True));
		self.flags.push(polling);
		self.main_variables.extend(self.flags.iter().cloned().chain(seen.into_iter().map(|(_, last)| last)));
	}

	/// What the listener does on a change: whenever and once keep a flag of their own
	fn check(&mut self, listener_word: ListenerWord, subject: Node, body: Node, value: &Node) -> Node {
		match listener_word {
			ListenerWord::Set | ListenerWord::Change => with_value(&body, value),
			ListenerWord::Whenever => {
				let (held, was) = (
					format!("{POLLING_PREFIX}{}_held", self.flags.len()),
					format!("{POLLING_PREFIX}{}_was", self.flags.len()),
				);
				self.flags.extend([held.clone(), was.clone()]);
				crate::variable_signals::edge_check(&held, &was, subject, body)
			}
			ListenerWord::Once => {
				let fired = format!("{POLLING_PREFIX}{}_fired", self.flags.len());
				self.flags.push(fired.clone());
				let condition = key(key(Node::Empty, Op::Not, Node::Symbol(fired.clone())), Op::And, subject);
				if_then(condition, block(vec![assign_variable(&fired, Node::True), body]))
			}
		}
	}
}

/// Listeners inside functions on their parameters or on main-level variables become subscriptions made at run time
pub fn subscribe(program: Node) -> Node {
	let (program, mut reflected) = reflections(with_function_listeners(program));
	let main_variables = main_variables(&program);
	let mut subscriptions = Subscriptions { main_variables, fired: 0 };
	let program = subscriptions.rewrite(program);
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else {
		return program;
	};
	if !crate::variable_signals::is_statement_list(&bracket, &separator) {
		return program;
	}
	// P111: a main-level listener on a variable a function writes through `global x` listens on its cell (decision
	// watched-cells); one on main-level writes only stays variable_signals' check after each write
	let mut definitions = vec![];
	collect_definitions(&program, &mut definitions);
	let written_in_functions: HashSet<String> =
		definitions.iter().flat_map(|definition| definition.body.declared_globals::<Vec<String>>()).collect();
	reflected.extend(
		statements
			.iter()
			.filter_map(listener_parts)
			.map(|(listener_word, subject, _)| watched_names(listener_word, &subject))
			.filter(|watched| watched.iter().any(|name| written_in_functions.contains(name)))
			.flatten(),
	);
	// P128: a named listener `alarm = whenever t > 30 {…}` can be removed, so its variables are reflected too
	let named: Vec<(String, String)> = statements
		.iter()
		.filter_map(named_listener)
		.flat_map(|(name, listener)| {
			let watched = listener_parts(&listener)
				.map(|(listener_word, subject, _)| watched_names(listener_word, &subject))
				.unwrap_or_default();
			watched.into_iter().map(move |variable| (name.clone(), variable))
		})
		.collect();
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
	let listener = listeners::read_named(statement, &[ON_WORD, ONCE_WORD, WHENEVER_WORD])?;
	let unnamed = listener.unnamed();
	listener_parts(&unnamed)?;
	Some((listener.name?, unnamed))
}

/// `watch(s) := whenever s > 5 {…}` arrives as `(watch(s) := (whenever s) > 5) {…}`, its block a statement of its own:
/// the function `watch(s) := { whenever s > 5 {…} }` (card whenever-without)
fn with_function_listeners(node: Node) -> Node {
	let node = node.map_children(with_function_listeners);
	let Node::List(items, Bracket::None, _) = node.drop_meta() else {
		return node;
	};
	let [head, body] = items.as_slice() else { return node };
	let Node::Key(function, Op::Define, value) = head.drop_meta() else {
		return node;
	};
	let listener = Node::List(vec![value.as_ref().clone(), body.clone()], Bracket::None, Separator::Space);
	if listener_parts(&listener).is_none() {
		return node;
	}
	Node::Key(function.clone(), Op::Define, Box::new(block(vec![listener])))
}

/// The variables a listener watches: `on set x` and `on change x` the one, `whenever` and `once` those of the condition
fn watched_names(listener_word: ListenerWord, subject: &Node) -> Vec<String> {
	match (listener_word, subject.drop_meta()) {
		(ListenerWord::Set | ListenerWord::Change, _) => {
			Some(signal_name(subject)).filter(|name| !name.is_empty()).into_iter().collect()
		}
		_ => subject.symbols_read(),
	}
}

/// `remove alarm from listeners of t` (listeners already `signal_listeners(t)`): the listener's name and the variable
fn removal(statement: &Node) -> Option<(String, String)> {
	let Node::List(items, _, _) = statement.drop_meta() else {
		return None;
	};
	let [remove, name, from, list] = items.as_slice() else {
		return None;
	};
	let Node::List(call_items, _, _) = list.drop_meta() else {
		return None;
	};
	let [listeners, variable] = call_items.as_slice() else {
		return None;
	};
	let reads = word(remove) == REMOVE_WORD && word(from) == FROM_WORD && word(listeners) == SIGNAL_LISTENERS;
	reads.then(|| (word(name), word(variable)))
}

fn contains_removal(node: &Node) -> bool {
	any_part(node, |part| removal(part).is_some())
}

/// A removal inside a function or a block (`stop() := { remove b from listeners of t }`): the places it changes are
/// main-level variables, declared global where it runs
fn nested_removals(node: Node, named: &[(String, String)]) -> Node {
	if let Some((name, variable)) = removal(&node) {
		let globals = named
			.iter()
			.filter(|(_, watched)| *watched == variable)
			.map(|(other, _)| from_template(&format!("{GLOBAL_WORD} {}", index_name(other, &variable)), &[]));
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
	let shifts: String = named
		.iter()
		.filter(|(other, watched)| other != name && watched == variable)
		.map(|(other, _)| {
			let other = index_name(other, variable);
			format!("if {other} > {index} {{ {other} = {other} - 1 }}; ")
		})
		.collect();
	from_template(
		&format!(
			"if {index} >= 0 {{ {SIGNAL_LISTENERS_SET}({variable}, {WITHOUT_PLACEHOLDER}({variable}, {index})); {shifts}{index} = -1 }}"
		),
		&[],
	)
}

/// `signal·without(s, skip)`: the listeners of s but the one at skip
fn without_function() -> Node {
	from_template(
		&format!(
			"{WITHOUT_PLACEHOLDER}(s, skip) := {{ kept = []; i = 0; for f in {SIGNAL_LISTENERS}(s) {{ if i != skip {{ kept = kept + [f] }}; i += 1 }}; kept }}"
		),
		&[],
	)
}

/// `listeners of x` → `signal_listeners(x)`, `clear listeners of x` → `signal_listeners_set(x, ø)`; and the variables
/// reflected so
fn reflections(program: Node) -> (Node, HashSet<String>) {
	let mut reflected = HashSet::new();
	let objects = main_object_literals(&program).into_iter().map(|(name, _)| name).collect();
	let program = reflected_lists(program, &objects, &mut reflected);
	(program, reflected)
}

/// `x.listeners`, `listeners of x`: x
pub(crate) fn listeners_subject(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::Key(subject, Op::Dot, member) if member.is_symbol(LISTENERS_WORD) => Some(subject),
		_ => None,
	}
}

/// a map's own field `listeners` stays its field: `p = {listeners: 7}; p.listeners` is 7
fn reflected_lists(node: Node, objects: &HashSet<String>, reflected: &mut HashSet<String>) -> Node {
	let listeners_subject = |node| listeners_subject(node).filter(|subject| !objects.contains(&word(subject)));
	let mut listeners_of = |subject: &Node| {
		reflected.insert(word(subject));
		call(SIGNAL_LISTENERS, vec![subject.clone()])
	};
	if let Some(subject) = listeners_subject(&node) {
		return listeners_of(subject);
	}
	match node.drop_meta() {
		// C#'s `listeners of x -= alarm` is `remove alarm from listeners of x`
		Node::Key(target, Op::SubAssign, name) if let Some(subject) = listeners_subject(target) => {
			let listeners = listeners_of(subject);
			Node::List(
				vec![symbol(REMOVE_WORD), name.as_ref().clone(), symbol(FROM_WORD), listeners],
				Bracket::None,
				Separator::Space,
			)
		}
		Node::List(items, _, _)
			if let [clear, target] = items.as_slice()
				&& word(clear) == CLEAR_WORD
				&& let Some(subject) = listeners_subject(target) =>
		{
			reflected.insert(word(subject));
			call(SIGNAL_LISTENERS_SET, vec![subject.clone(), Node::Empty])
		}
		// a one-item phrase is its item: `func add(_ a: T)` reads its parameter `a`, not `(a)`
		_ => match children_rewritten(node, |child| reflected_lists(child, objects, reflected)) {
			Node::List(mut items, Bracket::None, _) if items.len() == 1 => items.remove(0),
			other => other,
		},
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
			let over_params: Vec<String> = loop_variables(&definition.body)
				.into_iter()
				.filter(|(_, list)| definition.params.contains(list))
				.map(|(variable, _)| variable)
				.collect();
			let subscribable = |name: &String| {
				definition.params.contains(name)
					|| over_params.contains(name)
					|| (self.main_variables.contains(name) && !locals.contains(name))
			};
			let fields = field_paths(&definition.body)
				.into_iter()
				.filter(|path| path.split(FIELD_JOINER).next().is_some_and(|object| subscribable(&object.to_string())));
			let subscribable: HashSet<String> =
				symbols(&definition.body).into_iter().filter(subscribable).chain(fields).collect();
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
		let watched: Vec<String> =
			watched_names(listener_word, &subject).into_iter().filter(|name| subscribable.contains(name)).collect();
		if watched.is_empty() {
			return None;
		}
		let mut statements = vec![];
		// `old` in `on set` / `on change` reads the listener closure's old value
		let body = match (listener_word, with_old(&body, &self.main_variables)) {
			(ListenerWord::Set | ListenerWord::Change, Some(with_old)) => with_old(&symbol(OLD_WORD)),
			_ => body,
		};
		let check = self.listener_check(listener_word, subject, body, &mut statements);
		// the listener shares the main-level variables it changes, as a named one does (P124)
		// the watched cells it captures are no globals
		let captured: Vec<&str> =
			[VALUE_WORD, OLD_WORD].into_iter().chain(watched.iter().map(String::as_str)).collect();
		let globals =
			crate::event_signals::global_declarations(std::slice::from_ref(&check), &self.main_variables, &captured);
		let listener = match name {
			Some(name) => listener_function(name, globals, check, &watched, &mut statements),
			None => listener_closure(globals, check),
		};
		for name in watched {
			let signal = signal_node(&name);
			let listeners = key(
				call(SIGNAL_LISTENERS, vec![signal.clone()]),
				Op::Add,
				Node::List(vec![listener.clone()], Bracket::Square, Separator::None),
			);
			statements.push(call(SIGNAL_LISTENERS_SET, vec![signal, listeners]));
		}
		Some(if statements.len() == 1 { statements.remove(0) } else { block(statements) })
	}

	/// What the listener does with the new `value`; whenever and once first make their flag cell in `statements`
	fn listener_check(
		&mut self,
		listener_word: ListenerWord,
		subject: Node,
		body: Node,
		statements: &mut Vec<Node>,
	) -> Node {
		let value = symbol(VALUE_WORD);
		match listener_word {
			ListenerWord::Set => with_value(&body, &value),
			ListenerWord::Change => {
				if_then(key(value.clone(), Op::Ne, symbol(OLD_WORD)), block(vec![with_value(&body, &value)]))
			}
			// P156: when the condition becomes true; the cell holds whether it held at the last write
			ListenerWord::Whenever => {
				let held_cell = Node::Symbol(self.flag_cell(HELD_PREFIX, statements));
				// cells hold nodes: compared, not read as conditions
				let not_before = key(symbol(WAS_WORD), Op::Eq, Node::False);
				let holds = key(call(CELL_GET, vec![held_cell.clone()]), Op::Eq, Node::True);
				let became_true = key(holds, Op::And, not_before);
				let parts = vec![
					assign_variable(WAS_WORD, call(CELL_GET, vec![held_cell.clone()])),
					call(CELL_SET, vec![held_cell, subject]),
					if_then(became_true, block(vec![body])),
				];
				Node::List(parts, Bracket::Round, Separator::Semicolon)
			}
			ListenerWord::Once => {
				let fired = Node::Symbol(self.flag_cell(FIRED_PREFIX, statements));
				let not_fired = key(call(CELL_GET, vec![fired.clone()]), Op::Eq, Node::False);
				let condition = key(not_fired, Op::And, subject);
				if_then(condition, block(vec![call(CELL_SET, vec![fired, Node::True]), body]))
			}
		}
	}

	/// `signal·fired·0 = cell_new(false)`: a new flag cell of a listener, its name
	fn flag_cell(&mut self, prefix: &str, statements: &mut Vec<Node>) -> String {
		let flag = format!("{prefix}{}", self.fired);
		self.fired += 1;
		statements.push(assign_variable(&flag, call(CELL_NEW, vec![Node::False])));
		flag
	}
}

/// `(value, signal·old) => {check; 0}`: every listener closure of one arity returns one kind (wasm_emitter/functions/closures.rs)
fn listener_closure(globals: Vec<Node>, check: Node) -> Node {
	// a sequence, not a block: the declarations move to the main level, and a block of the check alone reads as data
	let closure_check = Node::List(globals.into_iter().chain([check]).collect(), Bracket::Round, Separator::Semicolon);
	from_template(
		&format!("({VALUE_WORD}, {OLD_PLACEHOLDER}) => {{ {CHECK_PLACEHOLDER}; 0 }}"),
		&[(CHECK_PLACEHOLDER, closure_check)],
	)
}

/// A named listener (P128) is a function of that name, and remembers where it sits in each list
fn listener_function(
	name: &str,
	globals: Vec<Node>,
	check: Node,
	watched: &[String],
	statements: &mut Vec<Node>,
) -> Node {
	let head = call(name, vec![symbol(VALUE_WORD), symbol(OLD_WORD)]);
	let body = globals.into_iter().chain([check, crate::node::int(0)]).collect();
	statements.push(key(head, Op::Define, block(body)));
	for variable in watched {
		let place = call(COUNT_WORD, vec![call(SIGNAL_LISTENERS, vec![signal_node(variable)])]);
		statements.push(assign_variable(&index_name(name, variable), place));
	}
	symbol(name)
}

/// The escaping variables become signals: made at their first main-level write, set and read through their cell
pub fn lower(program: Node) -> Node {
	let mut definitions = vec![];
	collect_definitions(&program, &mut definitions);
	let Escaping { signal_params, list_params, field_params, signal_lists, escaping } =
		escaping(&program, &definitions);
	if signal_params.is_empty() && escaping.is_empty() && field_params.is_empty() {
		return program;
	}
	let mut signals =
		Signals { signal_params, list_params, field_params, signal_lists, escaping, made: HashSet::new() };
	let program = signals.rewrite(program, &signals.escaping.clone(), true);
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else {
		return program;
	};
	Node::List(std::iter::once(set_function()).chain(statements).collect(), bracket, separator)
}

struct Signals {
	/// (function, parameter index) of each parameter that takes a signal
	signal_params: HashSet<(String, usize)>,
	/// (function, parameter index) of each parameter that takes a list of signals (`for s in xs {on change s …}`)
	list_params: HashSet<(String, usize)>,
	/// (function, parameter index, field) of each parameter whose field holds a signal (`on change p.age …`)
	field_params: HashSet<(String, usize, String)>,
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
			let inner = self.signals_inside(&definition);
			let body = self.rewrite(definition.body.clone(), &inner, false);
			return with_body(node, body);
		}
		// `xs.add(v)` on a signal is a write of its cell, which runs the listeners
		if let Node::Key(list, Op::Dot, method) = node.drop_meta() {
			if signals.contains(&signal_name(list)) && crate::list_mutations::updates_list(list, method) {
				let update = crate::list_mutations::list_update(list.clone(), method);
				return self.rewrite(update, signals, main);
			}
		}
		if matches!(node.drop_meta(), Node::Key(_, Op::Dot, _)) && signals.contains(&signal_name(&node)) {
			return call(CELL_GET, vec![node]);
		}
		match node {
			Node::Symbol(ref name) if signals.contains(name) => call(CELL_GET, vec![node]),
			// `for s in xs {…}` over signals: s names the signal, its uses read it
			Node::List(items, bracket, separator)
				if loop_variable(&items).is_some_and(|variable| signals.contains(&variable)) =>
			{
				let items = items
					.into_iter()
					.enumerate()
					.map(|(index, item)| if index == 1 { item } else { self.rewrite(item, signals, main) })
					.collect();
				Node::List(items, bracket, separator)
			}
			// `watched = [a, b]` of a list of signals keeps the signals
			Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(name) if self.signal_lists.contains(name)) => {
				Node::Key(target, Op::Assign, Box::new(self.signals_kept(*value, signals, main)))
			}
			// `person = {age: a}` of an object whose fields are signals keeps them there
			Node::Key(target, Op::Assign, value) if !fields_of(&word(&target), signals).is_empty() => {
				let object = word(&target);
				let fields = fields_of(&object, signals);
				if main {
					self.made.extend(fields.iter().map(|field| field_path(&object, field)));
				}
				Node::Key(target, Op::Assign, Box::new(self.fields_kept(*value, &fields, signals, main)))
			}
			Node::Key(target, op, value) if op.is_write() && signals.contains(&signal_name(&target)) => {
				self.signal_written(*target, op, *value, signals, main)
			}
			Node::Key(keyword, Op::Colon, name) if word(&keyword) == GLOBAL_WORD => Node::Key(keyword, Op::Colon, name),
			// a function changes an escaping variable's cell, never the variable: its `global x` goes
			Node::List(items, bracket, separator) if items.iter().any(|item| declares_global_of(item, signals)) => {
				let kept = items.into_iter().filter(|item| !declares_global_of(item, signals));
				Node::List(kept.map(|item| self.rewrite(item, signals, main)).collect(), bracket, separator)
			}
			Node::Key(left, op @ (Op::Dot | Op::Colon), right)
				if !matches!(op, Op::Dot) || matches!(right.drop_meta(), Node::Symbol(_)) =>
			{
				Node::Key(Box::new(self.rewrite(*left, signals, main)), op, right)
			}
			Node::List(items, bracket, separator) if self.is_signal_call(&items, signals) => {
				let arguments = self.signal_call_arguments(items, signals, main);
				Node::List(arguments, bracket, separator)
			}
			other => children_rewritten(other, |child| self.rewrite(child, signals, main)),
		}
	}

	/// The signals inside a function: the escaping variables it does not shadow, its parameters that take a signal, the
	/// variables of its loops over a list of signals, and the signal fields of its parameters
	fn signals_inside(&self, definition: &Definition) -> HashSet<String> {
		let locals = assigned_names(&definition.body);
		let globals: HashSet<String> = definition.body.declared_globals();
		let mut inner: HashSet<String> = self
			.escaping
			.iter()
			.filter(|name| !definition.params.contains(name) && (!locals.contains(*name) || globals.contains(*name)))
			.cloned()
			.collect();
		inner.extend(
			definition
				.params
				.iter()
				.enumerate()
				.filter(|(index, _)| self.signal_params.contains(&(definition.name.clone(), *index)))
				.map(|(_, name)| name.clone()),
		);
		let lists: Vec<&String> = definition
			.params
			.iter()
			.enumerate()
			.filter(|(index, _)| self.list_params.contains(&(definition.name.clone(), *index)))
			.map(|(_, name)| name)
			.collect();
		inner.extend(
			loop_variables(&definition.body)
				.into_iter()
				.filter(|(_, list)| lists.contains(&list))
				.map(|(variable, _)| variable),
		);
		inner.extend(
			self.field_params
				.iter()
				.filter(|(function, _, _)| *function == definition.name)
				.filter_map(|(_, index, field)| definition.params.get(*index).map(|param| field_path(param, field))),
		);
		inner
	}

	/// A write of a signal: of its cell, which runs the listeners; the first main-level write of an escaping variable
	/// makes its signal
	fn signal_written(&mut self, target: Node, op: Op, value: Node, signals: &HashSet<String>, main: bool) -> Node {
		let name = signal_name(&target);
		let value = match op {
			Op::Assign => self.rewrite(value, signals, main),
			Op::Inc | Op::Dec => key(
				call(CELL_GET, vec![target.clone()]),
				if op == Op::Inc { Op::Add } else { Op::Sub },
				crate::node::int(1),
			),
			_ => key(call(CELL_GET, vec![target.clone()]), op.base_op(), self.rewrite(value, signals, main)),
		};
		if main && self.escaping.contains(&name) && self.made.insert(name) {
			return assign(target, call(SIGNAL_NEW, vec![value]));
		}
		call(SET_FUNCTION, vec![target, value])
	}

	/// The arguments of a call that passes signals as they are: a subscription's signal, a signal or list of them, an
	/// object with signal fields for parameters that take them
	fn signal_call_arguments(&mut self, items: Vec<Node>, signals: &HashSet<String>, main: bool) -> Vec<Node> {
		let function = word(&items[0]);
		let subscribes = function == SIGNAL_LISTENERS_SET || function == SIGNAL_LISTENERS;
		items
			.into_iter()
			.enumerate()
			.map(|(index, item)| {
				if index == 0 || (subscribes && index == 1) {
					return item;
				}
				if subscribes {
					return self.rewrite(item, signals, main);
				}
				let param = (function.clone(), index - 1);
				if self.list_params.contains(&param) {
					return match item.drop_meta() {
						Node::Symbol(name) if self.signal_lists.contains(name) || signals.contains(name) => item,
						_ => self.signals_kept(item, signals, main),
					};
				}
				if self.signal_params.contains(&param) {
					return match item.drop_meta() {
						Node::Symbol(name) if signals.contains(name) => item,
						_ => call(SIGNAL_NEW, vec![self.rewrite(item, signals, main)]),
					};
				}
				let fields: HashSet<String> = self
					.field_params
					.iter()
					.filter(|(name, param, _)| *name == function && *param == index - 1)
					.map(|(_, _, field)| field.clone())
					.collect();
				match fields.is_empty() {
					true => self.rewrite(item, signals, main),
					false => self.fields_kept(item, &fields, signals, main),
				}
			})
			.collect()
	}

	/// A call that passes signals as they are: a subscription, or a function taking a signal parameter
	fn is_signal_call(&self, items: &[Node], signals: &HashSet<String>) -> bool {
		let Some(first) = items.first() else { return false };
		let function = word(first);
		let subscribes = (function == SIGNAL_LISTENERS_SET || function == SIGNAL_LISTENERS)
			&& items.get(1).is_some_and(|signal| signals.contains(&signal_name(signal)));
		let takes = |params: &HashSet<(String, usize)>| params.iter().any(|(name, _)| *name == function);
		let takes_fields = self.field_params.iter().any(|(name, _, _)| *name == function);
		subscribes || (items.len() > 1 && (takes(&self.signal_params) || takes(&self.list_params) || takes_fields))
	}

	/// An object literal whose `fields` are signals: a signal named there stays the signal, any other value becomes one
	fn fields_kept(&mut self, object: Node, fields: &HashSet<String>, signals: &HashSet<String>, main: bool) -> Node {
		let mut kept = |item: Node| match item.drop_meta().clone() {
			Node::Key(field, Op::Colon, value) if fields.contains(&word(&field)) => {
				let value = match value.drop_meta() {
					Node::Symbol(name) if signals.contains(name) => *value,
					_ => call(SIGNAL_NEW, vec![self.rewrite(*value, signals, main)]),
				};
				Node::Key(field, Op::Colon, Box::new(value))
			}
			_ => self.rewrite(item, signals, main),
		};
		match object.drop_meta().clone() {
			Node::List(items, Bracket::Curly, separator) => {
				Node::List(items.into_iter().map(&mut kept).collect(), Bracket::Curly, separator)
			}
			pair @ Node::Key(_, Op::Colon, _) => kept(pair),
			// `{age: 1}` of one entry arrives as its map (one_entry_maps.rs)
			Node::List(items, _, _) if items.len() == 2 && word(&items[0]) == crate::one_entry_maps::MAP_OF_ENTRY => {
				call(
					crate::one_entry_maps::MAP_OF_ENTRY,
					vec![self.fields_kept(items[1].clone(), fields, signals, main)],
				)
			}
			other => self.rewrite(other, signals, main),
		}
	}

	/// A list literal of signals: its signal names stay the signals, everything else is rewritten as usual
	fn signals_kept(&mut self, list: Node, signals: &HashSet<String>, main: bool) -> Node {
		match list.drop_meta().clone() {
			Node::List(items, Bracket::Square, separator) => {
				let items = items
					.into_iter()
					.map(|item| match item.drop_meta() {
						Node::Symbol(name) if signals.contains(name) => item,
						_ => self.rewrite(item, signals, main),
					})
					.collect();
				Node::List(items, Bracket::Square, separator)
			}
			other => self.rewrite(other, signals, main),
		}
	}
}

/// What escapes into signals
#[derive(Default)]
struct Escaping {
	signal_params: HashSet<(String, usize)>,
	list_params: HashSet<(String, usize)>,
	field_params: HashSet<(String, usize, String)>,
	signal_lists: HashSet<String>,
	escaping: HashSet<String>,
}

/// The parameters that take a signal or a list of them, and the main-level variables that escape: subscribed to inside
/// a function, passed for such a parameter (through further calls too), or kept in a list passed for one
fn escaping(program: &Node, definitions: &[Definition]) -> Escaping {
	let mut found = Escaping::default();
	// the main level subscribes or reflects (`listeners of x`) too
	visit_main_level(program, &mut |node| {
		let Node::List(items, _, _) = node else { return };
		if items.len() >= 2 && [SIGNAL_LISTENERS, SIGNAL_LISTENERS_SET].contains(&word(&items[0]).as_str()) {
			found.escaping.insert(signal_name(&items[1]));
		}
	});
	for definition in definitions {
		found.subscribed_in(definition);
	}
	let main_lists = main_list_literals(program);
	let main_objects = main_object_literals(program);
	loop {
		let before = found.size();
		// a variable kept in a field that is a signal becomes one: `person = {age: a}` with `person.age` watched
		let kept_in_fields: Vec<String> = main_objects
			.iter()
			.flat_map(|(object, literal)| {
				field_values(literal)
					.into_iter()
					.filter(|(field, _)| found.escaping.contains(&field_path(object, field)))
					.map(|(_, value)| value)
			})
			.collect();
		found.escaping.extend(kept_in_fields);
		visit_main_level(program, &mut |node| found.passed(None, node, &main_lists));
		for definition in definitions {
			definition.body.visit(&mut |node| found.passed(Some(definition), node, &main_lists));
		}
		if found.size() == before {
			return found;
		}
	}
}

impl Escaping {
	/// How much was found so far: no growth means all is found
	fn size(&self) -> [usize; 5] {
		[
			self.signal_params.len(),
			self.list_params.len(),
			self.field_params.len(),
			self.signal_lists.len(),
			self.escaping.len(),
		]
	}

	/// The signals the function subscribes to or reflects: its parameters, the lists its loops go over, fields of its
	/// parameters, or else main-level variables
	fn subscribed_in(&mut self, definition: &Definition) {
		let loops = loop_variables(&definition.body);
		let position = |name: &str| definition.params.iter().position(|param| param == name);
		definition.body.visit(&mut |node| {
			let Node::List(items, _, _) = node else { return };
			let subscribes = (items.len() == 3 && word(&items[0]) == SIGNAL_LISTENERS_SET)
				|| (items.len() == 2 && word(&items[0]) == SIGNAL_LISTENERS);
			if !subscribes {
				return;
			}
			let name = signal_name(&items[1]);
			if let Some((object, field)) = name.split_once(FIELD_JOINER) {
				match position(object) {
					Some(index) => self.field_params.insert((definition.name.clone(), index, field.to_string())),
					None => self.escaping.insert(name.clone()),
				};
				return;
			}
			let over = loops.iter().find(|(variable, _)| *variable == name).map(|(_, list)| list.clone());
			match (position(&name), over.and_then(|list| position(&list))) {
				(Some(index), _) => self.signal_params.insert((definition.name.clone(), index)),
				(None, Some(index)) => self.list_params.insert((definition.name.clone(), index)),
				(None, None) => self.escaping.insert(name),
			};
		});
	}

	/// The arguments of a call passed for a parameter that takes a signal, a list of them or an object with a signal
	/// field: the caller's own parameter takes one too, a main-level variable escapes
	fn passed(&mut self, scope: Option<&Definition>, node: &Node, main_lists: &[(String, Node)]) {
		let Node::List(items, _, _) = node else { return };
		let Some(function) = items.first().map(word) else {
			return;
		};
		for (index, argument) in items.iter().skip(1).enumerate() {
			let key = (function.clone(), index);
			let fields: Vec<String> = self
				.field_params
				.iter()
				.filter(|(name, param, _)| *name == function && *param == index)
				.map(|(_, _, field)| field.clone())
				.collect();
			for field in fields {
				self.field_passed(scope, argument, field);
			}
			if self.signal_params.contains(&key) {
				if let Node::Symbol(name) = argument.drop_meta() {
					self.signal_passed(scope, name.clone());
				}
			}
			if self.list_params.contains(&key) {
				for name in self.list_passed(scope, argument, main_lists) {
					self.signal_passed(scope, name);
				}
			}
		}
	}

	/// A signal passed: for the caller's parameter, which then takes one too, or a main-level variable that escapes
	fn signal_passed(&mut self, scope: Option<&Definition>, name: String) {
		match param_of(scope, &name) {
			Some(param) => self.signal_params.insert(param),
			None => self.escaping.insert(name),
		};
	}

	/// An object passed whose field is a signal: the field of the caller's parameter or variable, or of a literal's
	/// variable
	fn field_passed(&mut self, scope: Option<&Definition>, argument: &Node, field: String) {
		match argument.drop_meta() {
			Node::Symbol(name) => match param_of(scope, name) {
				Some((scope_function, position)) => self.field_params.insert((scope_function, position, field)),
				None => self.escaping.insert(field_path(name, &field)),
			},
			literal => {
				for (_, value) in field_values(literal).into_iter().filter(|(name, _)| *name == field) {
					self.signal_passed(scope, value);
				}
				false
			}
		};
	}

	/// A list of signals passed: the caller's parameter takes one too; the items named by a main-level list or a literal
	fn list_passed(
		&mut self,
		scope: Option<&Definition>,
		argument: &Node,
		main_lists: &[(String, Node)],
	) -> Vec<String> {
		match argument.drop_meta() {
			Node::Symbol(name) => match param_of(scope, name) {
				Some(param) => {
					self.list_params.insert(param);
					vec![]
				}
				None => {
					self.signal_lists.insert(name.clone());
					main_lists
						.iter()
						.filter(|(list, _)| list == name)
						.flat_map(|(_, literal)| named_items(literal))
						.collect()
				}
			},
			literal => named_items(literal),
		}
	}
}

/// The function of `scope` and the position of its parameter `name`
fn param_of(scope: Option<&Definition>, name: &str) -> Option<(String, usize)> {
	let definition = scope?;
	let position = definition.params.iter().position(|param| param == name)?;
	Some((definition.name.clone(), position))
}

/// The variables a list literal names as its items: `[a, b]` → a, b
fn named_items(list: &Node) -> Vec<String> {
	match list.drop_meta() {
		Node::List(items, Bracket::Square, _) => {
			items.iter().filter_map(|item| item.symbol_name().map(String::from)).collect()
		}
		_ => vec![],
	}
}

/// The main-level `xs = [a, b]` assignments: each variable and its list literal
fn main_list_literals(program: &Node) -> Vec<(String, Node)> {
	main_literals(program, |value| matches!(value.drop_meta(), Node::List(_, Bracket::Square, _)).then_some(value))
}

/// The main-level `x = {…}` assignments: each variable and its object literal
fn main_object_literals(program: &Node) -> Vec<(String, Node)> {
	main_literals(program, object_literal)
}

/// The main-level `x = literal` assignments whose value `literal_of` accepts: each variable and that literal
fn main_literals(program: &Node, literal_of: impl Fn(&Node) -> Option<&Node>) -> Vec<(String, Node)> {
	let mut literals = vec![];
	visit_main_level(program, &mut |node| {
		if let Node::Key(target, Op::Assign, value) = node {
			if let (Some(name), Some(literal)) = (target.symbol_name(), literal_of(value)) {
				literals.push((name.to_string(), literal.clone()));
			}
		}
	});
	literals
}

/// An object literal `{age: 1, …}`, also of one entry, which arrives as its map `map_of_entry({age: 1})` (one_entry_maps.rs)
fn object_literal(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		literal @ (Node::List(_, Bracket::Curly, _) | Node::Key(_, Op::Colon, _)) => Some(literal),
		Node::List(items, _, _) if items.len() == 2 && word(&items[0]) == crate::one_entry_maps::MAP_OF_ENTRY => {
			object_literal(&items[1])
		}
		_ => None,
	}
}

/// The fields of an object literal that hold a variable: `{age: a}` → (age, a)
fn field_values(object: &Node) -> Vec<(String, String)> {
	let pairs = match object.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.iter().collect(),
		pair => vec![pair],
	};
	pairs
		.into_iter()
		.filter_map(|pair| match pair.drop_meta() {
			Node::Key(field, Op::Colon, value) => Some((word(field), value.symbol_name()?.to_string())),
			_ => None,
		})
		.collect()
}

/// The signal a node names: a variable `x`, or a field `p.age` by its path
fn signal_name(node: &Node) -> String {
	match node.drop_meta() {
		Node::Key(object, Op::Dot, field) => match (object.drop_meta(), field.drop_meta()) {
			(Node::Symbol(object), Node::Symbol(field)) => field_path(object, field),
			_ => String::new(),
		},
		other => word(other),
	}
}

fn field_path(object: &str, field: &str) -> String {
	format!("{object}{FIELD_JOINER}{field}")
}

/// The node of a signal's name: `p.age` the field read
fn signal_node(name: &str) -> Node {
	match name.split_once(FIELD_JOINER) {
		Some((object, field)) => key(symbol(object), Op::Dot, symbol(field)),
		None => Node::Symbol(name.to_string()),
	}
}

/// The fields of `object` that are signals
fn fields_of(object: &str, signals: &HashSet<String>) -> HashSet<String> {
	signals
		.iter()
		.filter_map(|name| name.split_once(FIELD_JOINER))
		.filter(|(name, _)| *name == object)
		.map(|(_, field)| field.to_string())
		.collect()
}

/// The field paths `p.age` a node reads
fn field_paths(node: &Node) -> Vec<String> {
	let mut paths = vec![];
	node.visit(&mut |part| {
		if matches!(part, Node::Key(_, Op::Dot, _)) {
			paths.push(signal_name(part));
		}
	});
	paths.retain(|path| !path.is_empty());
	paths
}

/// `for s in xs {…}`: the loop variable s
fn loop_variable(items: &[Node]) -> Option<String> {
	if items.len() == 5 {
		for_in_variable(items)
	} else {
		None
	}
}

/// Each `for s in xs {…}` in the body: the loop variable and the name of the list
fn loop_variables(body: &Node) -> Vec<(String, String)> {
	let mut loops = vec![];
	body.visit(&mut |node| {
		if let Node::List(items, _, _) = node {
			if let (Some(variable), Some(Node::Symbol(list))) =
				(loop_variable(items), items.get(3).map(Node::drop_meta))
			{
				loops.push((variable, list.clone()));
			}
		}
	});
	loops
}

/// `signal·set(s, v)`: store v, then call each listener with the new and the old value; gives the new value
fn set_function() -> Node {
	from_template(
		&format!(
			"{SET_PLACEHOLDER}(s, v) := {{ {OLD_PLACEHOLDER} = {CELL_GET}(s); {CELL_SET}(s, v); for {LISTENER_PLACEHOLDER} in {SIGNAL_LISTENERS}(s) {{ {CLOSURE_CALL}({LISTENER_PLACEHOLDER}, {CELL_GET}(s), {OLD_PLACEHOLDER}) }}; {CELL_GET}(s) }}"
		),
		&[],
	)
}

/// The code parsed, its placeholders replaced: the generated names (`signal·old`) and the given nodes
fn from_template(code: &str, nodes: &[(&str, Node)]) -> Node {
	let names = [
		(SET_PLACEHOLDER, SET_FUNCTION),
		(OLD_PLACEHOLDER, OLD_WORD),
		(LISTENER_PLACEHOLDER, LISTENER_WORD),
		(WITHOUT_PLACEHOLDER, WITHOUT_FUNCTION),
	];
	let names = names.iter().map(|(placeholder, name)| (*placeholder, symbol(name)));
	super::nodes::filled(code, names.chain(nodes.iter().map(|(placeholder, node)| (*placeholder, node.clone()))))
}

/// `f(a, b) := body`, `def f(a, b) {body}`, `def (f a b) {body}`
fn definition(node: &Node) -> Option<Definition> {
	let definition = function_head(node)?;
	let is_block = definition.body.is_block();
	let read = match definition.form {
		DefinitionForm::Operator(Op::Define) => definition.result.is_none() && is_round_head(definition.head),
		DefinitionForm::KeywordBlock => {
			matches!(definition.block_bracket, Some(Bracket::None | Bracket::Round)) && is_block
		}
		DefinitionForm::KeywordSpaced => is_block,
		_ => false,
	};
	read.then(|| Definition {
		name: definition.name.to_string(),
		params: definition.written_parameters.iter().flat_map(parameter_names).collect(),
		body: definition.body.clone(),
	})
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
		meta @ Node::Meta { .. } => meta.under_meta(|node| with_body(node, body)),
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

/// Visit what runs at the main level: the program without its function definitions (no copy of it: a big literal
/// copied at every level took gigabytes, card big-literals)
fn visit_main_level<'a>(node: &'a Node, action: &mut dyn FnMut(&'a Node)) {
	if definition(node).is_some() {
		return;
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => {
			action(node.drop_meta());
			visit_main_level(left, action);
			visit_main_level(right, action);
		}
		Node::List(items, _, _) => {
			action(node.drop_meta());
			items.iter().for_each(|item| visit_main_level(item, action));
		}
		other => other.visit(action),
	}
}

/// The names the main level assigns
fn main_variables(program: &Node) -> HashSet<String> {
	crate::event_signals::main_level_variables(&crate::variable_signals::main_statements(program).0)
}

/// The names a function body assigns (its locals, unless declared global), not those of the functions nested in it
fn assigned_names(body: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	visit_main_level(body, &mut |node| {
		if let Node::Key(target, op, _) = node {
			if let (true, Node::Symbol(name)) = (op.is_write(), target.drop_meta()) {
				names.insert(name.clone());
			}
		}
	});
	names
}

/// `global x` of one of the names
fn declares_global_of(statement: &Node, names: &HashSet<String>) -> bool {
	matches!(statement.drop_meta(), Node::Key(keyword, Op::Colon, name) if word(keyword) == GLOBAL_WORD && names.contains(&word(name)))
}
