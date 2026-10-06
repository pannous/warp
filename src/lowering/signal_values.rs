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

use crate::declarations::word;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::variable_signals::{assign, block, defines_function, if_then, is_call_head, listener_parts, symbols, with_value, ListenerWord};
use crate::wasm_emitter::cells::{CELL_GET, CELL_NEW, CELL_SET, SIGNAL_LISTENERS, SIGNAL_LISTENERS_SET, SIGNAL_NEW};
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
const GLOBAL_WORD: &str = "global";

/// A function definition: its name, parameter names and body
struct Definition {
	name: String,
	params: Vec<String>,
	body: Node,
}

/// `signal·seen·0`: the value of a watched variable when the shared listeners last looked
const SEEN_PREFIX: &str = "signal·seen·";
/// `signal·polling·0`: whether the shared listener was declared yet
const POLLING_PREFIX: &str = "signal·polling·";

/// Main-level listeners watching a `shared` value become checks of the function `on·shared()` the runtime polls
pub fn poll_shared(program: Node) -> Node {
	let shared: HashSet<String> = crate::shared_arrays::shared_names(&program).into_iter().collect();
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	if shared.is_empty() || !crate::variable_signals::is_statement_list(&bracket, &separator) {
		return program;
	}
	let mut main_variables = crate::event_signals::main_level_variables(&statements);
	let (mut lowered, mut checks, mut flags, mut seen_count) = (vec![], vec![], vec![], 0);
	for statement in statements {
		let parts = listener_parts(&statement).filter(|(_, subject, _)| symbols(subject).iter().any(|name| shared.contains(name)));
		let Some((listener_word, subject, body)) = parts else {
			lowered.push(statement);
			continue;
		};
		let polling = format!("{POLLING_PREFIX}{}", flags.len());
		let watched = unique(symbols(&subject).into_iter().filter(|name| shared.contains(name) || main_variables.contains(name)).collect());
		let seen: Vec<(String, String)> = watched.into_iter().map(|name| { seen_count += 1; (name, format!("{SEEN_PREFIX}{}", seen_count - 1)) }).collect();
		let changed = seen.iter().map(|(name, last)| Node::Key(Box::new(Node::Symbol(name.clone())), Op::Ne, Box::new(Node::Symbol(last.clone()))))
			.reduce(|either, next| Node::Key(Box::new(either), Op::Or, Box::new(next))).expect("a watched shared value");
		let value = seen.first().map(|(name, _)| Node::Symbol(name.clone())).expect("a watched value");
		let check = match listener_word {
			ListenerWord::Set | ListenerWord::Change => with_value(&body, &value),
			ListenerWord::Whenever => if_then(subject, block(vec![body])),
			ListenerWord::Once => {
				let fired = format!("{POLLING_PREFIX}{}·fired", flags.len());
				flags.push(fired.clone());
				let condition = Node::Key(Box::new(Node::Key(Box::new(Node::Empty), Op::Not, Box::new(Node::Symbol(fired.clone())))), Op::And, Box::new(subject));
				if_then(condition, block(vec![assign(&fired, Node::True), body]))
			}
		};
		let remember = seen.iter().map(|(name, last)| assign(last, Node::Symbol(name.clone())));
		checks.push(if_then(Node::Key(Box::new(Node::Symbol(polling.clone())), Op::And, Box::new(changed)), block(remember.clone().chain([check]).collect())));
		lowered.extend(remember);
		lowered.push(assign(&polling, Node::True));
		flags.push(polling);
		main_variables.extend(flags.iter().cloned().chain(seen.into_iter().map(|(_, last)| last)));
	}
	if checks.is_empty() {
		return program;
	}
	let handler = crate::event_signals::function_with_globals(crate::host::SHARED_HANDLER, false, &checks, &main_variables);
	let starts = flags.iter().map(|flag| assign(flag, Node::False));
	Node::List(starts.chain(std::iter::once(handler)).chain(lowered).collect(), bracket, separator)
}

/// Listeners inside functions on their parameters or on main-level variables become subscriptions made at run time
pub fn subscribe(program: Node) -> Node {
	let main_variables = main_variables(&program);
	Subscriptions { main_variables, fired: 0 }.rewrite(program)
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
			let subscribable = |name: &String| definition.params.contains(name) || (self.main_variables.contains(name) && !locals.contains(name));
			let subscribable: HashSet<String> = symbols(&definition.body).into_iter().filter(subscribable).collect();
			let body = self.subscriptions(definition.body.clone(), &subscribable);
			return with_body(node, body);
		}
		map_children(node, &mut |child| self.rewrite(child))
	}

	/// The listeners in a function body that watch a subscribable variable, as subscriptions
	fn subscriptions(&mut self, node: Node, subscribable: &HashSet<String>) -> Node {
		if definition(&node).is_some() {
			return self.rewrite(node);
		}
		if let Some(subscription) = self.subscription(&node, subscribable) {
			return subscription;
		}
		map_children(node, &mut |child| self.subscriptions(child, subscribable))
	}

	/// `on change s {body}` → `signal_listeners_set(s, signal_listeners(s) + [(value, signal·old) => {if value != signal·old {body}; 0}])`,
	/// one per watched variable; a once listener first makes its flag cell
	fn subscription(&mut self, statement: &Node, subscribable: &HashSet<String>) -> Option<Node> {
		let (listener_word, subject, body) = listener_parts(statement)?;
		let watched: Vec<String> = match (listener_word, subject.drop_meta()) {
			(ListenerWord::Set | ListenerWord::Change, Node::Symbol(name)) => vec![name.clone()],
			(ListenerWord::Set | ListenerWord::Change, _) => return None,
			_ => unique(symbols(&subject)),
		};
		let watched: Vec<String> = watched.into_iter().filter(|name| subscribable.contains(name)).collect();
		if watched.is_empty() {
			return None;
		}
		let value = Node::Symbol(VALUE_WORD.to_string());
		let mut statements = vec![];
		let check = match listener_word {
			ListenerWord::Set => with_value(&body, &value),
			ListenerWord::Change => if_then(Node::Key(Box::new(value.clone()), Op::Ne, Box::new(Node::Symbol(OLD_WORD.to_string()))), block(vec![with_value(&body, &value)])),
			ListenerWord::Whenever => if_then(subject, block(vec![body])),
			ListenerWord::Once => {
				let fired = format!("{FIRED_PREFIX}{}", self.fired);
				self.fired += 1;
				statements.push(assign(&fired, call(CELL_NEW, vec![Node::False])));
				let not_fired = Node::Key(Box::new(call(CELL_GET, vec![Node::Symbol(fired.clone())])), Op::Eq, Box::new(Node::False));
				let condition = Node::Key(Box::new(not_fired), Op::And, Box::new(subject));
				if_then(condition, block(vec![call(CELL_SET, vec![Node::Symbol(fired), Node::True]), body]))
			}
		};
		// every listener closure of one arity returns one kind (wasm_emitter/closures.rs): 0
		let listener = from_template(&format!("({VALUE_WORD}, {OLD_PLACEHOLDER}) => {{ {CHECK_PLACEHOLDER}; 0 }}"), &[(CHECK_PLACEHOLDER, check)]);
		for name in watched {
			let signal = Node::Symbol(name);
			let listeners = Node::Key(Box::new(call(SIGNAL_LISTENERS, vec![signal.clone()])), Op::Add, Box::new(Node::List(vec![listener.clone()], Bracket::Square, Separator::None)));
			statements.push(call(SIGNAL_LISTENERS_SET, vec![signal, listeners]));
		}
		Some(if statements.len() == 1 { statements.remove(0) } else { block(statements) })
	}
}

/// The escaping variables become signals: made at their first main-level write, set and read through their cell
pub fn lower(program: Node) -> Node {
	let mut definitions = vec![];
	collect_definitions(&program, &mut definitions);
	let (signal_params, escaping) = escaping(&program, &definitions);
	if signal_params.is_empty() && escaping.is_empty() {
		return program;
	}
	let mut signals = Signals { signal_params, escaping, made: HashSet::new() };
	let program = signals.rewrite(program, &signals.escaping.clone(), true);
	let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
	Node::List(std::iter::once(set_function()).chain(statements).collect(), bracket, separator)
}

struct Signals {
	/// (function, parameter index) of each parameter that takes a signal
	signal_params: HashSet<(String, usize)>,
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
			let body = self.rewrite(definition.body.clone(), &inner, false);
			return with_body(node, body);
		}
		match node {
			Node::Symbol(ref name) if signals.contains(name) => call(CELL_GET, vec![node]),
			Node::Key(target, op, value) if is_write(op) && matches!(target.drop_meta(), Node::Symbol(name) if signals.contains(name)) => {
				let name = target.drop_meta().name();
				let value = match op {
					Op::Assign => self.rewrite(*value, signals, main),
					Op::Inc | Op::Dec => Node::Key(Box::new(call(CELL_GET, vec![*target.clone()])), if op == Op::Inc { Op::Add } else { Op::Sub }, Box::new(crate::node::int(1))),
					_ => Node::Key(Box::new(call(CELL_GET, vec![*target.clone()])), op.base_op(), Box::new(self.rewrite(*value, signals, main))),
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
					_ if self.signal_params.contains(&(function.clone(), index - 1)) => match item.drop_meta() {
						Node::Symbol(name) if signals.contains(name) => item,
						_ => call(SIGNAL_NEW, vec![self.rewrite(item, signals, main)]),
					},
					_ => self.rewrite(item, signals, main),
				}).collect();
				Node::List(arguments, bracket, separator)
			}
			other => map_children(other, &mut |child| self.rewrite(child, signals, main)),
		}
	}

	/// A call that passes signals as they are: a subscription, or a function taking a signal parameter
	fn is_signal_call(&self, items: &[Node], signals: &HashSet<String>) -> bool {
		let Some(first) = items.first() else { return false };
		let function = word(first);
		let subscribes = (function == SIGNAL_LISTENERS_SET || function == SIGNAL_LISTENERS) && items.get(1).is_some_and(|signal| signals.contains(&word(signal)));
		subscribes || (items.len() > 1 && self.signal_params.iter().any(|(name, _)| *name == function))
	}
}

/// The parameters that take a signal, and the main-level variables that escape: subscribed to inside a function, or
/// passed for such a parameter (through further calls too)
fn escaping(program: &Node, definitions: &[Definition]) -> (HashSet<(String, usize)>, HashSet<String>) {
	let mut signal_params = HashSet::new();
	let mut escaping = HashSet::new();
	for definition in definitions {
		definition.body.visit(&mut |node| {
			let Node::List(items, _, _) = node else { return };
			if items.len() == 3 && word(&items[0]) == SIGNAL_LISTENERS_SET {
				let name = word(&items[1]);
				match definition.params.iter().position(|param| *param == name) {
					Some(index) => { signal_params.insert((definition.name.clone(), index)); }
					None => { escaping.insert(name); }
				}
			}
		});
	}
	loop {
		let before = (signal_params.len(), escaping.len());
		let mut passed = |scope: Option<&Definition>, node: &Node| {
			let Node::List(items, _, _) = node else { return };
			let Some(function) = items.first().map(word) else { return };
			for (index, argument) in items.iter().skip(1).enumerate() {
				let Node::Symbol(name) = argument.drop_meta() else { continue };
				if !signal_params.contains(&(function.clone(), index)) {
					continue;
				}
				match scope.and_then(|definition| definition.params.iter().position(|param| param == name).map(|position| (definition, position))) {
					Some((definition, position)) => { signal_params.insert((definition.name.clone(), position)); }
					None => { escaping.insert(name.clone()); }
				}
			}
		};
		without_definitions(program).visit(&mut |node| passed(None, node));
		for definition in definitions {
			definition.body.visit(&mut |node| passed(Some(definition), node));
		}
		if (signal_params.len(), escaping.len()) == before {
			return (signal_params, escaping);
		}
	}
}

/// `signal·set(s, v)`: store v, then call each listener with the new and the old value; gives the new value
fn set_function() -> Node {
	from_template(&format!("{SET_PLACEHOLDER}(s, v) := {{ {OLD_PLACEHOLDER} = {CELL_GET}(s); {CELL_SET}(s, v); for {LISTENER_PLACEHOLDER} in {SIGNAL_LISTENERS}(s) {{ {CLOSURE_CALL}({LISTENER_PLACEHOLDER}, {CELL_GET}(s), {OLD_PLACEHOLDER}) }}; {CELL_GET}(s) }}"), &[])
}

/// The code parsed, its placeholders replaced: the generated names (`signal·old`) and the given nodes
fn from_template(code: &str, nodes: &[(&str, Node)]) -> Node {
	let names = [(SET_PLACEHOLDER, SET_FUNCTION), (OLD_PLACEHOLDER, OLD_WORD), (LISTENER_PLACEHOLDER, LISTENER_WORD)];
	let bindings = names.iter().map(|(placeholder, name)| (placeholder.to_string(), Node::Symbol(name.to_string())))
		.chain(nodes.iter().map(|(placeholder, node)| (placeholder.to_string(), node.clone()))).collect();
	crate::law::substitute(&crate::wasp_parser::parse(code), &bindings).drop_meta().clone()
}

fn call(function: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(function.to_string())], arguments].concat(), Bracket::Round, Separator::None)
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
	map_children(node.clone(), &mut |child| without_definitions(&child))
}

fn map_children(node: Node, rewrite: &mut impl FnMut(Node) -> Node) -> Node {
	match node {
		Node::Key(left, op, right) => Node::Key(Box::new(rewrite(*left)), op, Box::new(rewrite(*right))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(&mut *rewrite).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(rewrite(*node)), data },
		other => other,
	}
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
