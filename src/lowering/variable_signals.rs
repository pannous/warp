//! Signal keywords on variables (wiki/signal.md): `once x==5 {…}` runs its body the first time its condition holds
//! after a change of a variable the condition reads, `whenever x>1 {…}` each time. A module runs on one thread, so the
//! listener is its check after every later write of such a variable in the statements that follow it, loop bodies
//! included: a write inside an expression (`while x-->0`) becomes `(x--; check; x+1)`, keeping the expression's value.
//! `on set x {print value}` runs after every write of x; `value` (or `signal`, `event`) is the new value.
//! `on change x {…}` runs after a write that changed the value of x. A `:=` value is a derived signal (notes/signals.md):
//! a listener on it watches the variables its definition reads, transitively.
//! P111: a function writing a watched main-level variable through `global x` runs the listener too: the listener's
//! check becomes the function `signal·check·0()`, guarded by `signal_listening_0` (set where the listener is
//! declared, so writes before it are not seen), and each such write in a function body calls it.
//! `after tested: print "ok"` (or `after test`) runs after every later statement that calls the function test,
//! `before test {…}` before it.

use crate::declarations::{handler_parts, word};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const ONCE_WORD: &str = "once";
const WHENEVER_WORD: &str = "whenever";
const ON_WORD: &str = "on";
const SET_WORD: &str = "set";
const CHANGE_WORD: &str = "change";
const AFTER_WORD: &str = "after";
const BEFORE_WORD: &str = "before";
/// `tested`, `saved`: a function named in the past tense
const PAST_SUFFIXES: [&str; 2] = ["ed", "d"];
/// The written value inside an `on set` listener
const VALUE_WORDS: [&str; 3] = ["value", "signal", "event"];
/// `once_fired_0`: whether the first once listener ran
const FIRED_PREFIX: &str = "once_fired_";
/// `change_last_0`: the value the first change listener saw last
const LAST_PREFIX: &str = "change_last_";
/// `signal·check·0()`: the check of a listener, called after a write in a function (P111)
const CHECK_PREFIX: &str = "signal·check·";
/// `signal_listening_0`: whether the listener was declared yet
const LISTENING_PREFIX: &str = "signal_listening_";
const GLOBAL_WORD: &str = "global";

#[derive(Clone)]
enum Trigger {
	/// a write of one of these variables
	Write(HashSet<String>),
	/// a statement calling the function
	Call { function: String, before: bool },
}

#[derive(Clone)]
struct Listener {
	trigger: Trigger,
	/// None for `on set` and calls: every time
	condition: Option<Node>,
	body: Node,
	/// the flag of a once listener, None for whenever
	fired: Option<String>,
	/// the statement that starts the listener where it is declared
	start: Option<Node>,
}

impl Listener {
	fn watches(&self, name: &str) -> bool {
		self.watched().is_some_and(|watched| watched.contains(name))
	}

	fn watched(&self) -> Option<&HashSet<String>> {
		match &self.trigger {
			Trigger::Write(watched) => Some(watched),
			Trigger::Call { .. } => None,
		}
	}

	fn check(&self) -> Node {
		let Some(condition) = &self.condition else { return block(vec![self.body.clone()]) };
		match &self.fired {
			None => if_then(condition.clone(), block(vec![self.body.clone()])),
			Some(fired) => {
				let not_fired = Node::Key(Box::new(Node::Empty), Op::Not, Box::new(Node::Symbol(fired.clone())));
				let condition = Node::Key(Box::new(not_fired), Op::And, Box::new(condition.clone()));
				if_then(condition, block(vec![assign(fired, Node::True), self.body.clone()]))
			}
		}
	}
}

pub fn lower(node: Node) -> Node {
	let main_statements = match node.drop_meta() {
		Node::List(items, bracket, separator) if is_statement_list(bracket, separator) => items.clone(),
		_ => vec![],
	};
	let mut signals = Signals {
		count: 0,
		functions: defined_functions(&node),
		derived: derived_values(&node),
		declared_global: declared_globals(&node),
		main_variables: crate::event_signals::main_level_variables(&main_statements),
		depth: 0,
		remote: vec![],
	};
	let node = signals.lower(node, &[]);
	signals.remote_checks(node)
}

struct Signals {
	/// once and change listeners so far: each gets its own variable
	count: usize,
	/// the functions the program defines, which `after` and `before` may name
	functions: HashSet<String>,
	/// the `:=` values and the names their definitions read
	derived: HashMap<String, HashSet<String>>,
	/// the names some function declares `global`: their writes there are watched too (P111)
	declared_global: HashSet<String>,
	/// the variables the main level assigns, its generated flags included
	main_variables: HashSet<String>,
	/// how deep the statement lists nest: 1 is the main level
	depth: usize,
	/// the main-level listeners a function may trigger, each with its check function's number
	remote: Vec<Listener>,
}

impl Signals {
	fn lower(&mut self, node: Node, listeners: &[Listener]) -> Node {
		match node {
			Node::List(items, bracket, separator) if is_statement_list(&bracket, &separator) => {
				// checks added to a block of one statement (`{ x = i }`) make it a sequence: `{ x = i; check }`
				let count = items.len();
				let statements = self.statements(items, listeners);
				let separator = if statements.len() > count { Separator::Semicolon } else { separator };
				Node::List(statements, bracket, separator)
			}
			Node::List(items, bracket, separator) => {
				// a function body is not in the flow of the statements around it (P111: remote_checks)
				let listeners = if defines_function(&items) { &[] } else { listeners };
				Node::List(items.into_iter().map(|item| self.lower(item, listeners)).collect(), bracket, separator)
			}
			Node::Key(target, Op::Define, body) if is_call_head(&target) => Node::Key(target, Op::Define, Box::new(self.lower(*body, &[]))),
			Node::Key(target, op, value) => {
				let written = written_variable(&target, op).filter(|name| watches(listeners, name));
				let node = Node::Key(target, op, Box::new(self.lower(*value, listeners)));
				match written {
					Some(name) => {
						let value_after = value_after_write(&node);
						let mut parts = vec![node];
						parts.extend(checks(listeners, &name));
						parts.push(value_after);
						Node::List(parts, Bracket::Round, Separator::Semicolon)
					}
					None => match node {
						Node::Key(target, op, value) => Node::Key(Box::new(self.lower(*target, listeners)), op, value),
						_ => unreachable!("built as a key above"),
					},
				}
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.lower(*node, listeners)), data },
			other => other,
		}
	}

	/// A listener applies to the statements after it; a write as a whole statement is followed by the checks
	fn statements(&mut self, items: Vec<Node>, outer: &[Listener]) -> Vec<Node> {
		self.depth += 1;
		let mut listeners = outer.to_vec();
		let mut out = Vec::new();
		for item in items {
			if let Some(listener) = self.listener(&item) {
				out.extend(listener.start.clone());
				if self.depth == 1 && listener.watched().is_some_and(|watched| !watched.is_disjoint(&self.declared_global)) {
					out.extend(self.remote_listener(&listener));
				}
				listeners.push(listener);
				continue;
			}
			let (before, after) = call_listeners(&listeners, &item);
			out.extend(before);
			match statement_write(&item).filter(|name| watches(&listeners, name)) {
				Some(name) => {
					let Node::Key(target, op, value) = item.drop_meta().clone() else { unreachable!("a write is a key") };
					out.push(Node::Key(target, op, Box::new(self.lower(*value, &listeners))));
					out.extend(checks(&listeners, &name));
				}
				None => out.push(self.lower(item, &listeners)),
			}
			out.extend(after);
		}
		self.depth -= 1;
		out
	}

	/// The check function of a main-level listener and its activation: `signal·check·0() := {global …; if
	/// signal_listening_0 {check}}; signal_listening_0 = true` (the flag starts false in remote_checks)
	fn remote_listener(&mut self, listener: &Listener) -> Vec<Node> {
		let number = self.remote.len();
		let listening = format!("{LISTENING_PREFIX}{number}");
		self.main_variables.insert(listening.clone());
		let guarded = if_then(Node::Symbol(listening.clone()), block(vec![listener.check()]));
		let check = crate::event_signals::function_with_globals(&format!("{CHECK_PREFIX}{number}"), false, &[guarded], &self.main_variables);
		self.remote.push(listener.clone());
		vec![check, assign(&listening, Node::True)]
	}

	/// Each function writing a watched variable it declares `global` calls the listener's check after the write, and
	/// the listening flags start false at the top of the program
	fn remote_checks(&mut self, program: Node) -> Node {
		if self.remote.is_empty() {
			return program;
		}
		let program = self.function_bodies(program);
		let Node::List(statements, bracket, separator) = program.drop_meta().clone() else { return program };
		let flags = (0..self.remote.len()).map(|number| assign(&format!("{LISTENING_PREFIX}{number}"), Node::False));
		Node::List(flags.chain(statements).collect(), bracket, separator)
	}

	fn function_bodies(&mut self, node: Node) -> Node {
		match node {
			Node::Key(target, Op::Define, body) if is_call_head(&target) && !word(&head_word(&target)).starts_with(CHECK_PREFIX) => {
				let proxies = self.proxies(&body);
				Node::Key(target, Op::Define, Box::new(self.lower(*body, &proxies)))
			}
			Node::List(items, bracket, separator) if defines_function(&items) => {
				let proxies = self.proxies(&Node::List(items.clone(), bracket.clone(), separator.clone()));
				Node::List(items.into_iter().map(|item| self.lower(item, &proxies)).collect(), bracket, separator)
			}
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.function_bodies(item)).collect(), bracket, separator),
			Node::Key(left, op, right) => Node::Key(Box::new(self.function_bodies(*left)), op, Box::new(self.function_bodies(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.function_bodies(*node)), data },
			other => other,
		}
	}

	/// Listeners for a function body: each calls a check function after a write of a variable the body declares global
	fn proxies(&self, body: &Node) -> Vec<Listener> {
		let globals = declared_globals(body);
		self.remote.iter().enumerate().filter_map(|(number, listener)| {
			let watched: HashSet<String> = listener.watched()?.intersection(&globals).cloned().collect();
			let call = Node::List(vec![Node::Symbol(format!("{CHECK_PREFIX}{number}"))], Bracket::Round, Separator::None);
			(!watched.is_empty()).then(|| Listener { trigger: Trigger::Write(watched), condition: None, body: call, fired: None, start: None })
		}).collect()
	}

	/// `once x==5 {body}`, `whenever x>1 : body`, `on set x {body}`
	fn listener(&mut self, statement: &Node) -> Option<Listener> {
		let Node::List(items, _, _) = statement.drop_meta() else { return None };
		let keyword = word(items.first()?);
		let on_word = (keyword == ON_WORD).then(|| items.get(1).map(word)).flatten();
		if on_word.as_deref() == Some(SET_WORD) {
			return self.set_listener(&items[2..]);
		}
		if on_word.as_deref() == Some(CHANGE_WORD) {
			return self.change_listener(&items[2..]);
		}
		if keyword == AFTER_WORD || keyword == BEFORE_WORD {
			return self.call_listener(&items[1..], keyword == BEFORE_WORD);
		}
		if keyword != ONCE_WORD && keyword != WHENEVER_WORD {
			return None;
		}
		let (condition, body) = subject_and_body(&items[1..])?;
		let watched = self.sources(&condition);
		if watched.is_empty() {
			return None;
		}
		let fired = (keyword == ONCE_WORD).then(|| self.fresh_name(FIRED_PREFIX));
		let start = fired.as_ref().map(|fired| assign(fired, Node::False));
		let body = self.lower(body, &[]);
		Some(Listener { trigger: Trigger::Write(watched), condition: Some(condition), body, fired, start })
	}

	fn fresh_name(&mut self, prefix: &str) -> String {
		self.count += 1;
		let name = format!("{prefix}{}", self.count - 1);
		self.main_variables.insert(name.clone());
		name
	}

	/// The variables a write of which can change the expression: the names it reads, and through each `:=` value
	/// the names its definition reads
	fn sources(&self, expression: &Node) -> HashSet<String> {
		let mut sources = HashSet::new();
		let mut pending = symbols(expression);
		while let Some(name) = pending.pop() {
			if let Some(read) = self.derived.get(&name).filter(|_| !sources.contains(&name)) {
				pending.extend(read.iter().cloned());
			}
			sources.insert(name);
		}
		sources
	}

	/// `on set x {body}`: the body after every write of x, `value` in it is x; of a `:=` value, after each change
	fn set_listener(&mut self, rest: &[Node]) -> Option<Listener> {
		let (variable, body) = subject_and_body(rest)?;
		let Node::Symbol(name) = variable.drop_meta() else { return None };
		if self.derived.contains_key(name) {
			return self.change_listener(rest); // a `:=` value is never written: its sets are its changes
		}
		let bindings = VALUE_WORDS.iter().map(|value| (value.to_string(), Node::Symbol(name.clone()))).collect();
		let body = self.lower(crate::law::substitute(&body, &bindings), &[]);
		Some(Listener { trigger: Trigger::Write(HashSet::from([name.clone()])), condition: None, body, fired: None, start: None })
	}

	/// `on change x {body}`: the body after every write that changed x (a variable or a `:=` value), `value` in it is x
	fn change_listener(&mut self, rest: &[Node]) -> Option<Listener> {
		let (variable, body) = subject_and_body(rest)?;
		let variable = variable.drop_meta().clone();
		let Node::Symbol(name) = &variable else { return None };
		let last = self.fresh_name(LAST_PREFIX);
		let bindings = VALUE_WORDS.iter().map(|value| (value.to_string(), variable.clone())).collect();
		let body = self.lower(crate::law::substitute(&body, &bindings), &[]);
		let changed = Node::Key(Box::new(variable.clone()), Op::Ne, Box::new(Node::Symbol(last.clone())));
		Some(Listener {
			trigger: Trigger::Write(self.sources(&Node::Symbol(name.clone()))),
			condition: Some(changed),
			body: block(vec![assign(&last, variable.clone()), body]),
			fired: None,
			start: Some(assign(&last, variable)),
		})
	}

	/// `after tested: body`, `before test {body}`: test must be a function of the program
	fn call_listener(&mut self, rest: &[Node], before: bool) -> Option<Listener> {
		let (subject, body) = subject_and_body(rest)?;
		let function = self.function_named(&word(&subject))?;
		let body = self.lower(body, &[]);
		Some(Listener { trigger: Trigger::Call { function, before }, condition: None, body, fired: None, start: None })
	}

	/// `test`, `tested` (and `saved` for save, `stopped` for stop) name the defined function
	fn function_named(&self, name: &str) -> Option<String> {
		let mut stems = vec![name.to_string()];
		stems.extend(PAST_SUFFIXES.iter().filter_map(|suffix| name.strip_suffix(suffix)).map(str::to_string));
		if let Some(stem) = name.strip_suffix(PAST_SUFFIXES[0]) {
			let mut letters: Vec<char> = stem.chars().collect();
			if letters.len() > 1 && letters[letters.len() - 1] == letters[letters.len() - 2] {
				letters.pop(); // `stopped`: stop
				stems.push(letters.into_iter().collect());
			}
		}
		stems.into_iter().find(|stem| self.functions.contains(stem))
	}
}

/// The names of `f(x) := …`, `def f: …` and the other keyword definitions
fn defined_functions(node: &Node) -> HashSet<String> {
	let mut functions = HashSet::new();
	let head_name = |head: &Node| match head.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.first().map(word),
		other => Some(word(other)),
	};
	node.visit(&mut |part| match part {
		Node::Key(head, Op::Define, _) => functions.extend(head_name(head)),
		Node::List(items, _, _) if items.first().is_some_and(|first| crate::operators::is_function_keyword(&word(first))) => {
			if let Some(Node::Key(head, _, _)) = items.get(1).map(Node::drop_meta) {
				functions.extend(head_name(head));
			}
		}
		_ => {}
	});
	functions.remove("");
	functions
}

/// Each `name := expr` value (a getter, P71) and the names expr reads
fn derived_values(node: &Node) -> HashMap<String, HashSet<String>> {
	let mut derived = HashMap::new();
	node.visit(&mut |part| {
		if let (Some(name), Node::Key(_, _, body)) = (crate::getters::getter_name(part), part.drop_meta()) {
			derived.insert(name.to_string(), symbols(body).into_iter().collect());
		}
	});
	derived
}

pub(crate) fn symbols(node: &Node) -> Vec<String> {
	let mut names = vec![];
	node.visit(&mut |part| if let Node::Symbol(name) = part { names.push(name.clone()); });
	names
}

/// The names declared `global x` anywhere in the node
fn declared_globals(node: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	node.visit(&mut |part| if let Node::Key(keyword, Op::Colon, name) = part {
		if word(keyword) == GLOBAL_WORD {
			names.extend(Some(word(name)).filter(|name| !name.is_empty()));
		}
	});
	names
}

/// `f(x)` or `f()` as the head of a definition
fn is_call_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::List(_, Bracket::Round, _))
}

fn head_word(head: &Node) -> Node {
	match head.drop_meta() {
		Node::List(items, _, _) => items.first().cloned().unwrap_or(Node::Empty),
		other => other.clone(),
	}
}

/// `def f(x) {…}`, `fun f …`: a keyword definition
fn defines_function(items: &[Node]) -> bool {
	items.first().is_some_and(|first| crate::operators::is_function_keyword(&word(first)))
}

/// The bodies of the call listeners to run before and after the statement
fn call_listeners(listeners: &[Listener], statement: &Node) -> (Vec<Node>, Vec<Node>) {
	let (mut before, mut after) = (vec![], vec![]);
	for listener in listeners {
		if let Trigger::Call { function, before: runs_before } = &listener.trigger {
			if calls(statement, function) {
				if *runs_before { before.push(listener.check()) } else { after.push(listener.check()) }
			}
		}
	}
	(before, after)
}

/// Does the statement call the function, outside the blocks nested in it (their statements get their own listeners)
fn calls(node: &Node, function: &str) -> bool {
	match node.drop_meta() {
		Node::Symbol(name) => name == function,
		Node::List(items, bracket, separator) if !is_statement_list(bracket, separator) => {
			word(items.first().unwrap_or(&Node::Empty)) == function || items.iter().skip(1).any(|item| calls_inside(item, function))
		}
		Node::Key(left, _, right) => calls_inside(left, function) || calls_inside(right, function),
		_ => false,
	}
}

/// A call inside an expression: a bare name there is a value, not a call
fn calls_inside(node: &Node, function: &str) -> bool {
	!matches!(node.drop_meta(), Node::Symbol(_)) && calls(node, function)
}

/// `subject {body}` or `subject: body`
fn subject_and_body(rest: &[Node]) -> Option<(Node, Node)> {
	match rest {
		[subject, body] if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => Some((subject.clone(), body.clone())),
		[handler] => handler_parts(handler),
		_ => None,
	}
}

pub(crate) fn is_statement_list(bracket: &Bracket, separator: &Separator) -> bool {
	*bracket == Bracket::Curly || matches!(separator, Separator::Semicolon | Separator::Newline)
}

fn watches(listeners: &[Listener], name: &str) -> bool {
	listeners.iter().any(|listener| listener.watches(name))
}

fn checks(listeners: &[Listener], name: &str) -> Vec<Node> {
	listeners.iter().filter(|listener| listener.watches(name)).map(Listener::check).collect()
}

/// The variable `x = …`, `x += …`, `x++`, `x--` writes
fn written_variable(target: &Node, op: Op) -> Option<String> {
	let writes = op == Op::Assign || op.is_compound_assign() || matches!(op, Op::Inc | Op::Dec);
	match target.drop_meta() {
		Node::Symbol(name) if writes => Some(name.clone()),
		_ => None,
	}
}

fn statement_write(statement: &Node) -> Option<String> {
	match statement.drop_meta() {
		Node::Key(target, op, _) => written_variable(target, *op),
		_ => None,
	}
}

/// What the write gave: `x--` the value before it, `x+1`; any other write the new value
fn value_after_write(write: &Node) -> Node {
	let Node::Key(target, op, _) = write else { unreachable!("a write is a key") };
	let undo = match op {
		Op::Dec => Op::Add,
		Op::Inc => Op::Sub,
		_ => return target.as_ref().clone(),
	};
	Node::Key(target.clone(), undo, Box::new(crate::node::int(1)))
}

pub(crate) fn if_then(condition: Node, body: Node) -> Node {
	let head = Node::Key(Box::new(Node::Empty), Op::If, Box::new(condition));
	Node::Key(Box::new(head), Op::Then, Box::new(body))
}

pub(crate) fn block(statements: Vec<Node>) -> Node {
	Node::List(statements, Bracket::Curly, Separator::Semicolon)
}

pub(crate) fn assign(name: &str, value: Node) -> Node {
	Node::Key(Box::new(Node::Symbol(name.to_string())), Op::Assign, Box::new(value))
}
