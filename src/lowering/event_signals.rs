//! Named event signals (wiki/signal.md, notes/signals.md phase 2, P110): `raise stop the machine{reason:"…"}` runs the
//! `on stop the machine {…}` handlers of the program in their order, `event` in them is the data (ø without). The
//! handlers of a name become one function `on·stop·the·machine(event)`, defined where the first of them is written and
//! declaring `global` the main-level variables they mention, so a raise inside a function reaches them too; each raise
//! of the name is its call. A raise whose name no handler listens to stays the exception (try catches it).
//! Page events (phase 7): `on click {…}` and `on key {…}` are kept without a raise and get the wrapper
//! `on·click·node([event])` (declarations::with_node_wrappers): the browser playground calls it when the page sees the
//! event (web/playground/worker.js); natively nothing does, which a warning says. System events (`on interrupt`) are
//! kept the same way: the runtime calls their handlers (notes/system_signals.md).

use super::words::{COUNT_WORD, FROM_WORD, OF_WORD, ON_WORD};
use super::nodes::{block, call, children_rewritten, if_then, key};
use crate::declarations::{handler_parts, word};
use crate::node::{symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::signal_values::ungrouped_reflection;
use crate::variable_signals::{assign, symbols};
use crate::warp_parser::parse;
use std::collections::{HashMap, HashSet};

/// `once alarm {…}`: the handler runs at the first raise only (Node's emitter.once, DOM `{once: true}`)
const ONCE_WORD: &str = "once";
/// `once_fired_0`: whether the once handler ran (a plain word: `global once_fired_0` is parsed)
const FIRED_PREFIX: &str = "once_fired_";
/// `h_listening`: whether the named handler h still listens (plain words: `global h_listening` is parsed)
const LISTENING_SUFFIX: &str = "_listening";
const REMOVE_WORD: &str = "remove";
const LISTENERS_WORD: &str = "listeners";
/// `on error of f {…}` catches the errors of f's calls
const ERROR_WORD: &str = "error";
const CLEAR_WORD: &str = "clear";
/// `alarm_handler_3`: the flag name of an unnamed handler of a cleared event
const HANDLER_SUFFIX: &str = "_handler_";
/// `tick_listeners`: the handlers of tick subscribed inside blocks, `tick_listener` one of them
const SUBSCRIBERS_SUFFIX: &str = "_listeners";
const SUBSCRIBER_SUFFIX: &str = "_listener";
const LOOP_WORDS: [&str; 4] = ["for", "while", "repeat", "loop"];
const LOOP_SUBSCRIPTION_TOPIC: &str = "handler-in-loop";
/// `emit alarm{level: 3}` runs the `on alarm` handlers and goes on; nobody listening, it does nothing (P163). `send`
/// without `to` is the same; `raise` and `throw` are errors only
const EMIT_WORDS: [&str; 2] = ["emit", "send"];
/// Other systems' words for emit, working with a got-it note naming it (a program defining one of them keeps it)
const EMIT_ALIASES: [&str; 3] = ["fire", "trigger", "signal"];
const EMIT_TOPIC: &str = "emit-word";
const UNHANDLED_TOPIC: &str = "unhandled-emit";
const RAISE_WORDS: [&str; 2] = ["raise", "throw"];
/// `remove (listeners of alarm)#1 from listeners of alarm`: the place removed, evaluated once
const REMOVED_PLACE: &str = "removed·listener";
/// `on set x` and `on change x` are variable listeners (variable_signals.rs)
const VARIABLE_LISTENER_WORDS: [&str; 2] = ["set", "change"];
pub const HANDLER_PREFIX: &str = "on·";
const NAME_JOINER: &str = "·";
/// A generated function as written, its name and globals filled in
const FUNCTION_TEMPLATE: &str = "handler(event) := {}";
const PARAMETERLESS_TEMPLATE: &str = "handler() := {}";
const TEMPLATE_NAME: &str = "handler";
pub(crate) const EVENT_WORD: &str = "event";
/// The event as raised, kept while a handler that changes its `event` runs before the next one
const RAISED_EVENT: &str = "raised_event";
/// The events of the page that call their handlers from outside the program
pub const PAGE_EVENTS: [&str; 3] = ["click", "key", "input"];
/// `click·1`: the page event of one element's handler (element_events.rs), the page event and the element's number
pub const ELEMENT_EVENT_JOINER: char = '·';

/// A page event (`click`), or one element's (`click·1`)
pub fn is_page_event(name: &str) -> bool {
	PAGE_EVENTS.contains(&name.split(ELEMENT_EVENT_JOINER).next().unwrap_or(name))
}
/// The output binding of a program with page events: its last line when that is a name, read anew after each handler
pub const PAGE_VALUE: &str = "page·value";
/// The bindings of the elements of shown markup that hold computed parts, `page·hole·<path>` (card web-fine-holes)
const PAGE_HOLE: &str = "page·hole";
/// The events the system raises: the runtime calls their handlers (notes/system_signals.md)
pub const SYSTEM_EVENTS: [&str; 2] = ["interrupt", "exit"];

pub fn lower(program: Node) -> Node {
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	let (statements, program) = match with_function_error_handlers(&statements) {
		Ok(Some(guarded)) => (guarded.clone(), Node::List(guarded, bracket.clone(), separator.clone())),
		Ok(None) => (statements, program),
		Err(error) => return error,
	};
	// an event whose listeners are named is handled even when nothing raises it yet (`count listeners of tick`), and
	// every handler of it gets a flag as a named one has, so any of them can be listed and removed (card event-handlers)
	let variables = main_level_variables(&statements);
	let listed: HashSet<String> = listed_events(&program).into_iter().filter(|event| !variables.contains(event)).collect();
	let verbs = emit_verbs(&program);
	let raised: HashSet<String> = emitted_names(&program, &verbs).into_iter().chain(listed.iter().cloned()).collect();
	// an emit only block handlers answer (scoped_handlers.rs) is handled though no `on` of the program is
	let block_handled: HashSet<String> = raised.iter().filter(|event| crate::scoped_handlers::has_block_handler(&program, event)).cloned().collect();
	// so does an event with a named handler: the name is the handler's place among all of the event's handlers
	let with_names = statements.iter().filter_map(named_handler).filter(|(listener, _)| listener.is_some()).map(|(_, (event, _, _))| event);
	let flagged: HashSet<String> = listed.into_iter().chain(with_names).collect();
	let statements = subscribed_in_blocks(statements, &raised);
	let kept = match without_unraised(&statements, &raised, &variables, &emitted_names(&program, &RAISE_WORDS.map(String::from))) {
		Ok(kept) => kept,
		Err(error) => return error,
	};
	let statements = kept;
	let mut flags: Vec<(String, Node)> = vec![];
	let mut named: Vec<(String, String)> = vec![];
	let mut user_named: Vec<(String, String)> = vec![];
	let handlers: Vec<(usize, String, Node)> = statements.iter().enumerate()
		.filter_map(|(index, statement)| named_handler(statement).map(|(listener, (name, body, once))| (index, listener, name, body, once)))
		.filter(|(_, _, name, _, _)| raised.contains(name) || is_page_event(name) || SYSTEM_EVENTS.contains(&name.as_str()))
		.map(|(index, listener, name, body, once)| {
			let body = if once { run_once(body, &mut flags) } else { body };
			user_named.extend(listener.iter().map(|listener| (listener.clone(), name.clone())));
			let listener = listener.or_else(|| flagged.contains(&name).then(|| format!("{}{HANDLER_SUFFIX}{index}", name.replace(' ', "_"))));
			let body = match listener {
				Some(listener) => listening(body, &listener, &name, &mut flags, &mut named),
				None => body,
			};
			(index, name, body)
		})
		.collect();
	if handlers.is_empty() {
		// a program with timers stays like one with page events (system_signals.rs, before this pass made them)
		// an emit nobody listens to does nothing
		let statements: Vec<Node> = statements.into_iter().map(|statement| emits_as_calls(statement, &HashMap::new(), &verbs, &block_handled)).collect();
		if !statements.iter().any(defines_timer) {
			return Node::List(statements, bracket, separator);
		}
		let main_variables = main_level_variables(&statements);
		return Node::List(with_output_binding(statements, &main_variables), bracket, separator);
	}
	let unnamed = |event: &str| handlers.iter().filter(|(_, name, _)| name == event).count() - named.iter().filter(|(_, other)| other == event).count();
	let listeners = Listeners {
		counts: handlers.iter().map(|(_, name, _)| (name.clone(), unnamed(name))).collect(),
		// a handler nothing emits is gone: its name binds nothing
		bound: statements.iter().flat_map(bound_names).collect(),
		named,
	};
	// `h = on alarm {…}`: h is its place among alarm's handlers, an entry of `listeners of alarm`
	flags.extend(user_named.iter().filter_map(|(listener, event)| listeners.place(listener, event).map(|place| (listener.clone(), Node::int(place as i64)))));
	let statements: Vec<Node> = statements.into_iter().map(|statement| reflected(statement, &listeners)).collect();
	// a handler may remove itself (`if taken == 2 {remove h from listeners of tick}`)
	let handlers: Vec<(usize, String, Node)> = handlers.into_iter().map(|(index, name, body)| (index, name, reflected(body, &listeners))).collect();
	if let Some(failure) = statements.iter().chain(handlers.iter().map(|(_, _, body)| body)).find_map(first_error) {
		return failure;
	}
	let statements: Vec<Node> = flags.iter().map(|(flag, initial)| assign(flag, initial.clone())).chain(statements).collect();
	let handlers: Vec<(usize, String, Node)> = handlers.into_iter().map(|(index, name, body)| (index + flags.len(), name, body)).collect();
	#[cfg(feature = "native")]
	if let Some((index, name, _)) = handlers.iter().find(|(_, name, _)| is_page_event(name) && !raised.contains(name) && !crate::pipeline::is_for_a_page() && !serves_its_page(&statements)) {
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
	if let Some(cycle) = emit_cycle(&bodies, &verbs) {
		let (index, _) = first_handler.iter().find(|(_, name)| **name == cycle[0]).expect("a cycle runs through handlers");
		let steps: Vec<String> = cycle.iter().zip(cycle.iter().cycle().skip(1)).map(|(from, to)| format!("on {from} emits {to}")).collect();
		let message = format!("endless emit loop: {}; emit one of them under a condition", steps.join(", "));
		return crate::diagnostic::Diagnostic::at(&statements[*index], message).into_error();
	}
	let handler_lines: HashSet<usize> = handlers.iter().map(|(index, _, _)| *index).collect();
	let lowered = statements.into_iter().enumerate().filter_map(|(index, statement)| match first_handler.get(&index) {
		Some(name) => Some(function_with_globals(&handler_function_name(name), reads_event(&bodies[name]), &each_its_event(&bodies[name]).iter().flat_map(statements_of).collect::<Vec<_>>(), &main_variables)),
		None if handler_lines.contains(&index) => None,
		None => Some(statement),
	});
	let lowered: Vec<Node> = lowered.map(|statement| emits_as_calls(statement, &bodies, &verbs, &block_handled)).collect();
	let page_handlers: std::collections::BTreeMap<String, usize> = bodies.iter().filter(|(event, _)| is_page_event(event)).map(|(event, body)| (handler_function_name(event), usize::from(reads_event(body)))).collect();
	let stays = !page_handlers.is_empty() || lowered.iter().any(defines_timer);
	let lowered = if stays { with_output_binding(lowered, &main_variables) } else { lowered };
	crate::declarations::with_node_wrappers(Node::List(lowered, bracket, separator), &page_handlers)
}

/// `on conect {…}` where nothing raises conect never runs (Node's emitter.on("conect") is silent): a warning, naming
/// a raised event one letter away, and the statements without it (it ran its body once, as a block). Page and system
/// events come from outside the program
fn without_unraised(statements: &[Node], raised: &HashSet<String>, variables: &HashSet<String>, errors: &HashSet<String>) -> Result<Vec<Node>, Node> {
	// `once ready {…}` of a variable is a variable listener (variable_signals.rs)
	let from_outside = |name: &str| is_page_event(name) || SYSTEM_EVENTS.contains(&name) || variables.contains(name);
	let unraised = |statement: &Node| named_handler(statement).map(|(_, (name, _, _))| name).filter(|name| !raised.contains(name) && !from_outside(name));
	let warnings: Vec<crate::diagnostic::Diagnostic> = statements.iter().filter_map(|statement| unraised(statement).map(|name| {
		let hint = match errors.contains(&name) {
			true => format!("; `raise {name}` is an error (P163), `emit {name}` sends the event"),
			false => crate::extensions::strings::near_miss(&name, raised.iter().cloned()).map(|near| format!("; did you mean {near}?")).unwrap_or_default(),
		};
		crate::diagnostic::Diagnostic::at(statement, format!("on {name}: nothing emits {name}, so this handler never runs{hint}"))
	})).collect();
	if !warnings.is_empty() {
		crate::diagnostic::report(&warnings)?;
	}
	Ok(statements.iter().filter(|statement| unraised(statement).is_none()).cloned().collect())
}

/// The output binding of a program the page keeps running (page events, timers): its last line, when that is a name,
/// is the function PAGE_VALUE too, which the page reads anew after each handler
fn with_output_binding(mut statements: Vec<Node>, main_variables: &HashSet<String>) -> Vec<Node> {
	// a name, markup (`div{ p{ "clicked " + count } }`, card web-element), or a choice between such
	// (`if users.loading then "Loading…" else users`, card web-async), which the page shows anew
	let shown = statements.last().map(Node::drop_meta).filter(|last| matches!(last, Node::Symbol(_)) || crate::markup::is_markup(last) || is_shown_choice(last)).cloned();
	if let Some(shown) = shown {
		// markup: each element holding a computed part is read on its own too, the page changes only those that differ
		let holes = crate::markup::holes(&shown).into_iter().map(|(path, element)| function_with_globals(&hole_name(&path), false, &[element], main_variables));
		let bindings: Vec<Node> = holes.chain([function_with_globals(PAGE_VALUE, false, &[shown], main_variables)]).collect();
		let last = statements.len() - 1;
		statements.splice(last..last, bindings);
	}
	statements
}

/// `page·hole·1·0`: the binding of the element at that path of the shown markup (worker.js reads the path back)
fn hole_name(path: &[usize]) -> String {
	path.iter().fold(PAGE_HOLE.to_string(), |name, index| format!("{name}{NAME_JOINER}{index}"))
}

/// `if c then a else b`, `c ? a : b`, `a ?? b`: a choice whose parts only read (no call, no write), so reading it anew
/// changes nothing
fn is_shown_choice(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Else | Op::Question | Op::Coalesce, _)) && only_reads(node)
}

/// Names, literals, markup, operators that compute (no assignment, no ++), and data lists of these
fn only_reads(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Symbol(_) | Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::Empty | Node::True | Node::False => true,
		markup if crate::markup::is_markup(markup) => true,
		Node::Key(left, op, right) => !matches!(op, Op::Assign | Op::Define | Op::Inc | Op::Dec | Op::While | Op::Do) && !op.is_compound_assign() && only_reads(left) && only_reads(right),
		Node::List(items, Bracket::Square | Bracket::Curly, _) => items.iter().all(only_reads),
		_ => false,
	}
}

/// `(on·every·0) := {…}`, the handler of a timer or channel listener system_signals.rs made, or `(on·fetch·0) := {…}`
/// of an async fetch (fetch_signals.rs)
fn defines_timer(statement: &Node) -> bool {
	let Node::Key(head, _, _) = statement.drop_meta() else { return false };
	let name = match head.drop_meta() {
		Node::List(items, _, _) => items.first().map(Node::drop_meta),
		other => Some(other),
	};
	matches!(name, Some(Node::Symbol(name)) if name.starts_with(crate::host::TIMER_HANDLER_PREFIX) || name.starts_with(crate::host::FETCH_HANDLER_PREFIX))
}

/// The body guarded by a fresh flag: `if once_fired_0 == false { once_fired_0 = true; body }`
fn run_once(body: Node, flags: &mut Vec<(String, Node)>) -> Node {
	let flag = format!("{FIRED_PREFIX}{}", flags.len());
	flags.push((flag.clone(), Node::False));
	let not_fired = key(Node::Symbol(flag.clone()), Op::Eq, Node::False);
	if_then(not_fired, block(vec![assign(&flag, Node::True), body]))
}

/// `for i in 1 to 3 { on tick {…} }`: a handler inside a block subscribes each time the block runs (Node's emitter.on):
/// `tick_listeners = tick_listeners + [event => body]`, and one main-level handler runs the subscribed ones. The
/// statements with `tick_listeners = []` and that handler first
fn subscribed_in_blocks(statements: Vec<Node>, raised: &HashSet<String>) -> Vec<Node> {
	let mut subscribed: Vec<String> = vec![];
	let statements: Vec<Node> = statements.into_iter().map(|statement| match handler(&statement) {
		Some(_) => statement,
		None => statement.map_children(|child| subscriptions(child, raised, &mut subscribed, false)),
	}).collect();
	let started = subscribed.iter().flat_map(|event| {
		let list = subscribers_name(event);
		let listener = format!("{}{SUBSCRIBER_SUFFIX}", event.replace(' ', "_"));
		[parse(&format!("{list} = []")), parse(&format!("on {event} {{ for {listener} in {list} {{ {listener}(event) }} }}"))]
	});
	started.chain(statements).collect()
}

/// A raised event's handler in a block, not in a function, becomes its subscription; one in a loop subscribes at each
/// pass (a listener leak, unless meant), which a got-it note says
fn subscriptions(node: Node, raised: &HashSet<String>, subscribed: &mut Vec<String>, in_loop: bool) -> Node {
	if let Some((event, body, false)) = handler(&node).filter(|(event, _, _)| raised.contains(event)) {
		if in_loop {
			let reason = format!("a handler in a loop subscribes once per pass: each emit {event} runs all of them; subscribe before the loop unless that is meant");
			crate::normalize::set_position_of(&node);
			crate::diagnostic::advise_once(LOOP_SUBSCRIPTION_TOPIC, &format!("on {event} {{…}}"), "on … before the loop", &reason);
		}
		let list = Node::Symbol(subscribers_name(&event));
		let listener = key(symbol(EVENT_WORD), Op::FatArrow, body);
		let added = key(list.clone(), Op::Add, Node::List(vec![listener], Bracket::Square, Separator::None));
		if !subscribed.contains(&event) {
			subscribed.push(event);
		}
		return key(list, Op::Assign, added);
	}
	let in_loop = in_loop || is_loop(&node);
	match node.drop_meta() {
		Node::Key(_, Op::Define, _) => node,
		_ => node.map_children(|child| subscriptions(child, raised, subscribed, in_loop)),
	}
}

/// `for i in xs {…}`, `while c {…}`, `repeat 3 {…}`
pub(crate) fn is_loop(node: &Node) -> bool {
	match node.drop_meta() {
		Node::Key(_, Op::While, _) => true,
		Node::Key(head, Op::Do, _) => is_loop(head),
		Node::List(items, _, _) => items.first().is_some_and(|first| LOOP_WORDS.contains(&word(first).as_str())),
		_ => false,
	}
}

fn subscribers_name(event: &str) -> String {
	format!("{}{SUBSCRIBERS_SUFFIX}", event.replace(' ', "_"))
}

/// `h = on alarm {body}` (Node's emitter.on returning a handle, P128): the handler and the name it can be removed by
fn named_handler(statement: &Node) -> Option<(Option<String>, (String, Node, bool))> {
	match statement.drop_meta() {
		Node::Key(target, Op::Assign, value) if matches!(target.drop_meta(), Node::Symbol(_)) => handler(&flattened(value)).map(|parts| (Some(word(target)), parts)),
		_ => handler(statement).map(|parts| (None, parts)),
	}
}

/// The value of `h = on alarm {…}` arrives nested, `on (alarm {…})`, `(on stop) (the (machine {…}))`: its words in
/// one row
fn flattened(node: &Node) -> Node {
	fn words(node: &Node) -> Vec<Node> {
		match node.drop_meta() {
			Node::List(items, Bracket::None, Separator::Space) => items.iter().flat_map(words).collect(),
			_ => vec![node.clone()],
		}
	}
	Node::List(words(node), Bracket::None, Separator::Space)
}

/// A named handler runs while its flag `h_listening` holds: `remove h from listeners of alarm` clears it
fn listening(body: Node, listener: &str, event: &str, flags: &mut Vec<(String, Node)>, named: &mut Vec<(String, String)>) -> Node {
	let flag = listening_flag(listener);
	flags.push((flag.clone(), Node::True));
	named.push((listener.to_string(), event.to_string()));
	if_then(Node::Symbol(flag), block(vec![body]))
}

fn listening_flag(listener: &str) -> String {
	format!("{listener}{LISTENING_SUFFIX}")
}

/// The handlers of the program's events as `listeners of alarm` reflects them (P128, card event-handlers)
struct Listeners {
	/// every handler with a flag, by its name (`h`, or `alarm_handler_3` of an unnamed one) and event, in order
	named: Vec<(String, String)>,
	/// the handlers without a flag, by event: they always listen
	counts: HashMap<String, usize>,
	/// the names the program binds (variables, loop variables): a removed word that is none of them is no listener
	bound: HashSet<String>,
}

impl Listeners {
	fn flags(&self, event: &str) -> Vec<String> {
		self.named.iter().filter(|(_, other)| other == event).map(|(listener, _)| listening_flag(listener)).collect()
	}

	/// The place of a named handler among its event's handlers, from 1
	fn place(&self, listener: &str, event: &str) -> Option<usize> {
		self.named.iter().filter(|(_, other)| other == event).position(|(name, _)| name == listener).map(|index| index + 1)
	}

	fn is_event(&self, event: &str) -> bool {
		self.counts.contains_key(event)
	}

	/// `remove x from listeners of alarm`: a named handler's flag cleared; a place (`2`, `(listeners of alarm)#1`, a
	/// loop variable) clears the flag at that place, nothing once removed; any other word is a loud error
	fn removal(&self, removed: &Node, event: &str) -> Node {
		let name = word(removed);
		if self.place(&name, event).is_some() {
			return assign(&listening_flag(&name), Node::False);
		}
		let listens_elsewhere = self.named.iter().any(|(listener, _)| *listener == name);
		if !name.is_empty() && (listens_elsewhere || !self.bound.contains(&name)) {
			return crate::node::error(&format!("{name} is no named listener of {event}"));
		}
		// evaluated once: `(listeners of alarm)#1` names another handler after the first removal
		let place = symbol(REMOVED_PLACE);
		let at_place = |(index, flag): (usize, String)| {
			let condition = key(place.clone(), Op::Eq, Node::int(index as i64 + 1));
			if_then(condition, block(vec![assign(&flag, Node::False)]))
		};
		let removal = std::iter::once(assign(REMOVED_PLACE, reflected(removed.clone(), self)));
		block(removal.chain(self.flags(event).into_iter().enumerate().map(at_place)).collect())
	}

	/// `listeners of alarm`: the places of its handlers still listening, `[1, 3]` once the second is removed
	fn list(&self, event: &str) -> Node {
		let entries: Vec<String> = self.flags(event).iter().enumerate().map(|(index, flag)| format!("(if {flag} then [{}] else [])", index + 1)).collect();
		match entries.is_empty() {
			true => parse("[]"),
			false => parse(&entries.join(" + ")),
		}
	}
}

/// `remove h from listeners of alarm` and C#'s `listeners of alarm -= h`: the handler's flag cleared; `count listeners
/// of alarm`: the handlers still listening; `listeners of alarm` anywhere else: their places. Listeners of variables
/// stay signal_values' (P128)
fn reflected(statement: Node, listeners: &Listeners) -> Node {
	let (items, bracket, separator) = match statement.drop_meta() {
		Node::List(items, bracket, separator) => (ungrouped_reflection(items.clone()), bracket.clone(), separator.clone()),
		_ => return statement.map_children(|child| reflected(child, listeners)),
	};
	let words: Vec<String> = items.iter().map(word).collect();
	let named = &listeners.named;
	match words.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
		[REMOVE_WORD, _, FROM_WORD, LISTENERS_WORD, OF_WORD, event @ ..] if listeners.is_event(&event.join(" ")) => listeners.removal(&items[1], &event.join(" ")),
		// `remove h from alarm`: short for the above unless alarm is also a variable (card g-_SgI)
		[REMOVE_WORD, _, FROM_WORD, event @ ..] if listeners.is_event(&event.join(" ")) && !listeners.bound.contains(&event.join(" ")) => listeners.removal(&items[1], &event.join(" ")),
		[LISTENERS_WORD, OF_WORD, _] => match items[2].drop_meta() {
			Node::Key(event, Op::SubAssign, removed) if listeners.is_event(&word(event)) => listeners.removal(removed, &word(event)),
			_ if listeners.is_event(&words[2]) => listeners.list(&words[2]),
			_ => statement,
		},
		[CLEAR_WORD, LISTENERS_WORD, OF_WORD, event @ ..] if named.iter().any(|(_, other)| *other == event.join(" ")) => {
			let event = event.join(" ");
			block(named.iter().filter(|(_, other)| *other == event).map(|(listener, _)| assign(&listening_flag(listener), Node::False)).collect())
		}
		[COUNT_WORD, LISTENERS_WORD, OF_WORD, event @ ..] if listeners.is_event(&event.join(" ")) => {
			let event = event.join(" ");
			let listening = listeners.flags(&event).into_iter().map(|flag| format!(" + (if {flag} then 1 else 0)"));
			parse(&format!("{}{}", listeners.counts[&event], listening.collect::<String>()))
		}
		[LISTENERS_WORD, OF_WORD, event @ ..] if listeners.is_event(&event.join(" ")) => listeners.list(&event.join(" ")),
		_ => match listed_inline(&items, listeners) {
			Some(inlined) => Node::List(inlined, bracket, separator).map_children(|child| reflected(child, listeners)),
			None => statement.map_children(|child| reflected(child, listeners)),
		},
	}
}

/// `for l in listeners of alarm {…}`: the words `listeners of alarm` inside a longer row become its list
fn listed_inline(items: &[Node], listeners: &Listeners) -> Option<Vec<Node>> {
	let words: Vec<String> = items.iter().map(word).collect();
	let start = (0..words.len().saturating_sub(2)).find(|at| words[*at] == LISTENERS_WORD && words[at + 1] == OF_WORD)?;
	let end = (start + 3..=words.len()).rev().find(|end| listeners.is_event(&words[start + 2..*end].join(" ")))?;
	let mut items = items.to_vec();
	items.splice(start..end, [listeners.list(&words[start + 2..end].join(" "))]);
	Some(items)
}

/// `on error of f {handler}` (card g_ADRM, wiki/Error.md: a catch outside a block catches for its function): every call
/// of f is `try body else handler`, its value the handler's when the body fails; the statement itself is gone
fn with_function_error_handlers(statements: &[Node]) -> Result<Option<Vec<Node>>, Node> {
	let handlers: Vec<(usize, String, Node)> = statements.iter().enumerate()
		.filter_map(|(index, statement)| function_error_handler(statement).map(|(function, handler)| (index, function, handler)))
		.collect();
	if handlers.is_empty() {
		return Ok(None);
	}
	let mut statements = statements.to_vec();
	for (_, function, handler) in &handlers {
		let definition = statements.iter().position(|statement| crate::variable_signals::defined_function_name(statement.drop_meta()) == *function);
		let guarded = definition.and_then(|index| guarded_definition(&statements[index], handler).map(|guarded| (index, guarded)));
		let Some((index, guarded)) = guarded else {
			return Err(crate::node::error(&format!("on error of {function}: the program defines no function {function}")));
		};
		statements[index] = guarded;
	}
	let handler_places: HashSet<usize> = handlers.iter().map(|(index, _, _)| *index).collect();
	Ok(Some(statements.into_iter().enumerate().filter(|(index, _)| !handler_places.contains(index)).map(|(_, statement)| statement).collect()))
}

/// `on error of f {handler}`, `on error of f: handler`: f and the handler
fn function_error_handler(statement: &Node) -> Option<(String, Node)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [on, error, of, rest @ ..] = items.as_slice() else { return None };
	if word(on) != ON_WORD || word(error) != ERROR_WORD || word(of) != OF_WORD {
		return None;
	}
	let (function, handler) = crate::variable_signals::subject_and_body(rest)?;
	Some((word(&function), handler)).filter(|(function, _)| !function.is_empty())
}

/// The definition with its body `try body else handler`: `f(x) := body`, `def f(x): body`, `fun f(x) {body}`
fn guarded_definition(definition: &Node, handler: &Node) -> Option<Node> {
	let guarded = |body: &Node| Node::List(vec![symbol(crate::warp_parser::TRY_MARKER), body.clone(), handler.clone()], Bracket::Round, Separator::Space);
	match definition.drop_meta() {
		Node::Key(head, op @ (Op::Define | Op::Assign | Op::Colon), body) => Some(Node::Key(head.clone(), *op, Box::new(guarded(body)))),
		Node::List(items, bracket, separator) if items.len() >= 2 => {
			let last = items.last()?;
			let body = match last.drop_meta() {
				Node::List(_, Bracket::Curly, _) => guarded(last),
				_ => guarded_definition(last, handler)?,
			};
			let mut items = items.clone();
			*items.last_mut()? = body;
			Some(Node::List(items, bracket.clone(), separator.clone()))
		}
		_ => None,
	}
}

/// The first error a lowering left in `node` (`h is no named listener of tick`)
fn first_error(node: &Node) -> Option<Node> {
	let mut found = None;
	node.visit(&mut |part| if found.is_none() && matches!(part, Node::Error(_)) {
		found = Some(part.clone());
	});
	found
}

/// The names a program binds: assigned ones and loop variables (`for l in …`)
fn bound_names(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |part| match part {
		Node::Key(target, Op::Assign | Op::Define, _) => {
			names.insert(word(target));
		}
		Node::List(items, _, _) => {
			if let [first, variable, ..] = items.as_slice() {
				if word(first) == "for" {
					names.insert(word(variable));
				}
			}
		}
		_ => {}
	});
	names
}

/// `on alarm {body}`, `on stop the machine {body}`, `on alarm: body`, `once alarm {body}`: the event's name, the body
/// and whether it runs once
fn handler(statement: &Node) -> Option<(String, Node, bool)> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let (on, rest) = items.split_first()?;
	let (last, words) = rest.split_last()?;
	let once = word(on) == ONCE_WORD;
	if !(once || word(on) == ON_WORD) || words.first().is_some_and(|first| VARIABLE_LISTENER_WORDS.contains(&word(first).as_str())) {
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
	Some((name, body, once))
}

/// The words that emit in this program: emit, send and the aliases it does not name itself (soft keywords, P165)
pub(crate) fn emit_verbs(program: &Node) -> Vec<String> {
	EMIT_WORDS.iter().chain(EMIT_ALIASES.iter()).filter(|verb| !crate::soft_keywords::program_names(program, verb)).map(|verb| verb.to_string()).collect()
}

/// `emit alarm`, `emit stop the machine{reason:"…"}`, `emit item 1` (or a `verbs` word): the name and the data (ø
/// without)
pub(crate) fn emitted(node: &Node, verbs: &[String]) -> Option<(String, Node)> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let (verb, rest) = items.split_first()?;
	let (last, words) = rest.split_last()?;
	if !verbs.contains(&word(verb)) {
		return None;
	}
	let (last_word, data) = match last.drop_meta() {
		Node::Key(name, Op::Colon, data) if matches!(data.drop_meta(), Node::List(_, Bracket::Curly, _)) => (name.as_ref().clone(), data.as_ref().clone()),
		// `emit item 1`, `emit said "hi"`: a value after the words is the data (EventEmitter's emit("item", 1))
		Node::Number(_) | Node::Text(_) | Node::Char(_) if !words.is_empty() => return Some((event_name(words.iter())?, last.clone())),
		_ => (last.clone(), Node::Empty),
	};
	Some((event_name(words.iter().chain([&last_word]))?, data))
}

/// The words of an event, joined by spaces; None unless all are plain words
fn event_name<'a>(words: impl Iterator<Item = &'a Node>) -> Option<String> {
	let words: Vec<String> = words.map(word).collect();
	(!words.is_empty() && words.iter().all(|word| !word.is_empty())).then(|| words.join(" "))
}

/// The events of `listeners of tick` anywhere: counted, cleared, listed or removed from (`count listeners of tick`)
fn listed_events(program: &Node) -> HashSet<String> {
	let mut events = HashSet::new();
	program.visit(&mut |part| if let Node::List(items, _, _) = part {
		let items = ungrouped_reflection(items.clone());
		let words: Vec<String> = items.iter().map(word).collect();
		for at in (0..words.len().saturating_sub(2)).filter(|at| words[*at] == LISTENERS_WORD && words[at + 1] == OF_WORD) {
			let event: Vec<String> = items[at + 2..].iter().map(|item| match item.drop_meta() {
				Node::Key(event, Op::SubAssign, _) => word(event),
				_ => word(item),
			}).take_while(|word| !word.is_empty()).collect();
			if !event.is_empty() {
				events.insert(event.join(" "));
			}
		}
	});
	events
}

fn emitted_names(program: &Node, verbs: &[String]) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |part| if let Some((name, _)) = emitted(part, verbs) { names.insert(name); });
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
	let statements: Vec<Node> = global_declarations(bodies, main_variables, &[EVENT_WORD]).into_iter().chain(bodies.iter().cloned()).collect();
	let template = if takes_event { FUNCTION_TEMPLATE } else { PARAMETERLESS_TEMPLATE };
	let Node::Key(head, op, _) = parse(template).drop_meta().clone() else { unreachable!("the template is a definition") };
	let name = symbol(name);
	let head = crate::law::substitute(&head, &HashMap::from([(TEMPLATE_NAME.to_string(), name)]));
	key(head, op, Node::List(statements, Bracket::Curly, Separator::Semicolon))
}

/// A program serving (serve_routes, lowering/serve.rs) serves its page too: the page raises its events
#[cfg(feature = "native")]
fn serves_its_page(statements: &[Node]) -> bool {
	statements.iter().any(|statement| matches!(statement.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.drop_meta().name() == crate::host::SERVE_ROUTES)))
}

/// `global n` for each main-level variable the bodies mention, except the `parameters` of the function they become
pub(crate) fn global_declarations(bodies: &[Node], main_variables: &HashSet<String>, parameters: &[&str]) -> Vec<Node> {
	let mut mentioned: Vec<String> = bodies.iter().flat_map(symbols).filter(|name| main_variables.contains(name) && !parameters.contains(&name.as_str())).collect();
	mentioned.sort();
	mentioned.dedup();
	mentioned.iter().map(|name| global_declaration(name)).collect()
}

/// The statements of a block, so the function's value is its last one's, not a one-item block; `{emit ask}` holds the words of its one statement
pub(crate) fn statements_of(block: &Node) -> Vec<Node> {
	match block.drop_meta() {
		Node::List(words, Bracket::Curly, Separator::Space) if words.len() > 1 => vec![Node::List(words.clone(), Bracket::None, Separator::Space)],
		Node::List(statements, Bracket::Curly, _) => statements.clone(),
		other => vec![other.clone()],
	}
}

/// `global name`, built rather than parsed: a generated name (`users·loading`) parses as a product
pub(crate) fn global_declaration(name: &str) -> Node {
	crate::law::substitute(&parse(&format!("global {TEMPLATE_NAME}")), &HashMap::from([(TEMPLATE_NAME.to_string(), symbol(name))]))
}

/// Each handler gets the event as raised (the DOM hands one mutable event object down the listeners): when a body
/// changes `event`, the next one starts with `event = raised_event`
fn each_its_event(bodies: &[Node]) -> Vec<Node> {
	let changes_event = |body: &Node| {
		let mut changes = false;
		body.visit(&mut |part| match part {
			Node::Key(target, op, _) if *op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec) => {
				changes |= crate::variable_signals::root_variable(target).is_some_and(|name| name == EVENT_WORD);
			}
			Node::Key(target, Op::Dot, call) => {
				let mutating = matches!(call.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|method| crate::analyzer::is_list_mutating_method(&word(method))));
				changes |= mutating && crate::variable_signals::root_variable(target).is_some_and(|name| name == EVENT_WORD);
			}
			_ => {}
		});
		changes
	};
	let changing_before_last = bodies.split_last().is_some_and(|(_, earlier)| earlier.iter().any(changes_event));
	if !changing_before_last {
		return bodies.to_vec();
	}
	// an event is an object, so a reference (P200b): kept and handed on as copies
	let copy_of = |name: &str| crate::library_words::copy_call(symbol(name), Node::False);
	let restored = |index: usize| (index > 0 && bodies[..index].iter().any(changes_event)).then(|| assign(EVENT_WORD, copy_of(RAISED_EVENT)));
	let kept = assign(RAISED_EVENT, copy_of(EVENT_WORD));
	std::iter::once(kept).chain(bodies.iter().enumerate().flat_map(|(index, body)| restored(index).into_iter().chain([body.clone()]))).collect()
}

/// Handlers take `event` only when a body reads it: an unread parameter has no type a caller from outside (the page)
/// could pass a value as
pub(crate) fn reads_event(bodies: &[Node]) -> bool {
	bodies.iter().flat_map(symbols).any(|name| name == EVENT_WORD)
}

/// `on a {emit b}; on b {emit a}` never ends (the stack overflows at run time): the first cycle of handlers that emit
/// the next one without a condition, `[a, b]`. A once handler or a named one runs under a flag, so it breaks a cycle
fn emit_cycle(bodies: &HashMap<String, Vec<Node>>, verbs: &[String]) -> Option<Vec<String>> {
	let edges: HashMap<String, Vec<String>> = bodies.iter().map(|(name, handlers)| {
		let emitted = handlers.iter().flat_map(|body| unconditional_emits(body, verbs)).filter(|next| bodies.contains_key(next));
		(name.clone(), emitted.collect())
	}).collect();
	let mut names: Vec<&String> = bodies.keys().collect();
	names.sort();
	let mut done = HashSet::new();
	names.into_iter().find_map(|name| cycle_from(name, &edges, &mut vec![], &mut done))
}

fn cycle_from(name: &str, edges: &HashMap<String, Vec<String>>, path: &mut Vec<String>, done: &mut HashSet<String>) -> Option<Vec<String>> {
	if let Some(start) = path.iter().position(|visited| visited == name) {
		return Some(path[start..].to_vec());
	}
	if !done.insert(name.to_string()) {
		return None;
	}
	path.push(name.to_string());
	let cycle = edges[name].iter().find_map(|next| cycle_from(next, edges, path, done));
	path.pop();
	cycle
}

/// The events a handler body emits at its top level, outside any condition or loop
fn unconditional_emits(body: &Node, verbs: &[String]) -> Vec<String> {
	if let Some((name, _)) = emitted(body, verbs) {
		return vec![name];
	}
	match body.drop_meta() {
		Node::List(statements, Bracket::Curly, _) => statements.iter().filter_map(|statement| emitted(statement, verbs).map(|(name, _)| name)).collect(),
		_ => vec![],
	}
}

/// Each `emit name{data}` with handlers is the call `on·name(data)`, `on·name()` when no handler reads `event`; one
/// nobody handles is ø, nothing
fn emits_as_calls(node: Node, handled: &HashMap<String, Vec<Node>>, verbs: &[String], block_handled: &HashSet<String>) -> Node {
	if let Some((name, data)) = emitted(&node, verbs) {
		let verb = match node.drop_meta() {
			Node::List(items, _, _) => items.first().map(word).unwrap_or_default(),
			_ => String::new(),
		};
		if EMIT_ALIASES.contains(&verb.as_str()) {
			crate::normalize::set_position_of(&node);
			crate::diagnostic::educate_once(EMIT_TOPIC, &verb, "emit", &format!("`emit {name}` sends an event to its handlers (P163)"));
		}
		let call = match handled.get(&name) {
			Some(bodies) => {
				let arguments = if reads_event(bodies) { vec![data] } else { vec![] };
				call(&handler_function_name(&name), arguments)
			}
			None => {
				// P202: an emit no handler anywhere receives does nothing, with a got-it note
				if !block_handled.contains(&name) {
					crate::normalize::set_position_of(&node);
					crate::diagnostic::advise_once(UNHANDLED_TOPIC, &format!("emit {name}"), &format!("on {name} {{…}}"), &format!("no handler for event {name}: this emit does nothing"));
				}
				Node::Empty
			}
		};
		// `{ emit alarm }` is a block of one statement: it stays a block
		let braced = matches!(node.drop_meta(), Node::List(_, Bracket::Curly, _));
		return if braced { Node::List(vec![call], Bracket::Curly, Separator::Semicolon) } else { call };
	}
	children_rewritten(node, |child| emits_as_calls(child, handled, verbs, block_handled))
}

/// The variables the main level assigns (`n = 0`, `n += 1`, `n: int = 0`)
pub(crate) fn main_level_variables(statements: &[Node]) -> HashSet<String> {
	statements.iter().filter_map(|statement| match statement.drop_meta() {
		Node::Key(target, op, _) if *op == Op::Assign || op.is_compound_assign() => match target.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			// `level: int = 0`
			Node::Key(name, Op::Colon, _) => matches!(name.drop_meta(), Node::Symbol(_)).then(|| word(name)),
			_ => None,
		},
		_ => None,
	}).collect()
}
