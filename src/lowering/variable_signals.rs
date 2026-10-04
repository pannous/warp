//! Signal keywords on variables (wiki/signal.md): `once x==5 {…}` runs its body the first time its condition holds
//! after a change of a variable the condition reads, `whenever x>1 {…}` each time. A module runs on one thread, so the
//! listener is its check after every later write of such a variable in the statements that follow it, loop bodies
//! included: a write inside an expression (`while x-->0`) becomes `(x--; check; x+1)`, keeping the expression's value.
//! `on set x {print value}` runs after every write of x; `value` (or `signal`, `event`) is the new value.
//! `after tested: print "ok"` (or `after test`) runs after every later statement that calls the function test,
//! `before test {…}` before it.

use crate::declarations::{handler_parts, word};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;

const ONCE_WORD: &str = "once";
const WHENEVER_WORD: &str = "whenever";
const ON_WORD: &str = "on";
const SET_WORD: &str = "set";
const AFTER_WORD: &str = "after";
const BEFORE_WORD: &str = "before";
/// `tested`, `saved`: a function named in the past tense
const PAST_SUFFIXES: [&str; 2] = ["ed", "d"];
/// The written value inside an `on set` listener
const VALUE_WORDS: [&str; 3] = ["value", "signal", "event"];
/// `once_fired_0`: whether the first once listener ran
const FIRED_PREFIX: &str = "once_fired_";

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
}

impl Listener {
	fn watches(&self, name: &str) -> bool {
		matches!(&self.trigger, Trigger::Write(watched) if watched.contains(name))
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
	let functions = defined_functions(&node);
	Signals { count: 0, functions }.lower(node, &[])
}

struct Signals {
	/// once listeners so far: each gets its own flag
	count: usize,
	/// the functions the program defines, which `after` and `before` may name
	functions: HashSet<String>,
}

impl Signals {
	fn lower(&mut self, node: Node, listeners: &[Listener]) -> Node {
		match node {
			Node::List(items, bracket, separator) if is_statement_list(&bracket, &separator) => {
				Node::List(self.statements(items, listeners), bracket, separator)
			}
			Node::List(items, bracket, separator) => {
				Node::List(items.into_iter().map(|item| self.lower(item, listeners)).collect(), bracket, separator)
			}
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
		let mut listeners = outer.to_vec();
		let mut out = Vec::new();
		for item in items {
			if let Some(listener) = self.listener(&item) {
				if let Some(fired) = &listener.fired {
					out.push(assign(fired, Node::False));
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
		out
	}

	/// `once x==5 {body}`, `whenever x>1 : body`, `on set x {body}`
	fn listener(&mut self, statement: &Node) -> Option<Listener> {
		let Node::List(items, _, _) = statement.drop_meta() else { return None };
		let keyword = word(items.first()?);
		if keyword == ON_WORD && items.get(1).map(word).as_deref() == Some(SET_WORD) {
			return self.set_listener(&items[2..]);
		}
		if keyword == AFTER_WORD || keyword == BEFORE_WORD {
			return self.call_listener(&items[1..], keyword == BEFORE_WORD);
		}
		if keyword != ONCE_WORD && keyword != WHENEVER_WORD {
			return None;
		}
		let (condition, body) = subject_and_body(&items[1..])?;
		let mut watched = HashSet::new();
		condition.visit(&mut |part| if let Node::Symbol(name) = part { watched.insert(name.clone()); });
		if watched.is_empty() {
			return None;
		}
		let fired = (keyword == ONCE_WORD).then(|| {
			self.count += 1;
			format!("{FIRED_PREFIX}{}", self.count - 1)
		});
		let body = self.lower(body, &[]);
		Some(Listener { trigger: Trigger::Write(watched), condition: Some(condition), body, fired })
	}

	/// `on set x {body}`: the body after every write of x, `value` in it is x
	fn set_listener(&mut self, rest: &[Node]) -> Option<Listener> {
		let (variable, body) = subject_and_body(rest)?;
		let Node::Symbol(name) = variable.drop_meta() else { return None };
		let bindings = VALUE_WORDS.iter().map(|value| (value.to_string(), Node::Symbol(name.clone()))).collect();
		let body = self.lower(crate::law::substitute(&body, &bindings), &[]);
		Some(Listener { trigger: Trigger::Write(HashSet::from([name.clone()])), condition: None, body, fired: None })
	}

	/// `after tested: body`, `before test {body}`: test must be a function of the program
	fn call_listener(&mut self, rest: &[Node], before: bool) -> Option<Listener> {
		let (subject, body) = subject_and_body(rest)?;
		let function = self.function_named(&word(&subject))?;
		let body = self.lower(body, &[]);
		Some(Listener { trigger: Trigger::Call { function, before }, condition: None, body, fired: None })
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
