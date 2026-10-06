//! System values (P136, notes/system_signals.md): `battery` (percent), `charging`, `online` and `dark mode` are the
//! machine's state, read by the host word `system_value("battery")` (a yes/no value is `system_value("online") != 0`).
//! A program that binds the name itself (`online = 3`, `f(online) := …`, a field `{battery: 5}`) keeps its own.
//! `name` (before signal_values::poll_shared) marks each read as the symbol `system·battery`, so poll_shared turns a
//! main-level listener on one (`whenever battery < 20% {…}`, `on change online {…}`) into a check of `on·shared`, like
//! a listener on a shared value, and adds a timer `on every 1000 ms {}` so a `warp run` stays and looks once a second.
//! `read` (after poll_shared) makes each marked symbol the host call.
//! `whenever battery < 20% {…}` arrives as `battery < (20 % {…})`: a percent compared with the battery is that number.

use crate::declarations::word;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::HashSet;
use warp_runtime::host_words::{BATTERY, SYSTEM_VALUE, SYSTEM_VALUES};

/// The mark of a system value read between the two passes
pub const SYSTEM_PREFIX: &str = "system·";
const LISTENER_WORDS: [&str; 3] = ["whenever", "once", "on"];
/// The timer that keeps a listening program and wakes it for the checks
const KEEP_LISTENING: &str = "on every 1000 ms { 0 }"; // { 0 }: an empty body fails (card empty-timer)

pub fn name(program: Node) -> Node {
	let bound = bound_names(&program);
	let names: Vec<(&str, bool)> = SYSTEM_VALUES.into_iter().filter(|(name, _)| !bound.contains(*name)).collect();
	if names.is_empty() {
		return program;
	}
	let marked = mark(program, &names);
	if marked_names(&marked).is_empty() {
		return marked;
	}
	// a program of one statement (`whenever dark mode {…}`) is a list of that one
	let (statements, bracket, separator) = match marked.drop_meta() {
		Node::List(items, bracket, separator) if crate::variable_signals::is_statement_list(bracket, separator) => (items.clone(), bracket.clone(), separator.clone()),
		single => (vec![single.clone()], Bracket::None, Separator::Newline),
	};
	let listens = |statement: &Node| LISTENER_WORDS.contains(&first_word(statement).as_str()) && !marked_names(statement).is_empty();
	let Some(first_listener) = statements.iter().position(listens) else { return marked };
	// right after the first listener: the program's last statement stays its value
	let mut statements: Vec<Node> = statements.into_iter().map(with_percent_body).collect();
	statements.insert(first_listener + 1, crate::wasp_parser::parse(KEEP_LISTENING));
	Node::List(statements, bracket, separator)
}

pub fn read(program: Node) -> Node {
	if marked_names(&program).is_empty() {
		return program;
	}
	reading(program)
}

/// The system values the node reads, marked
pub fn marked_names(node: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	node.visit(&mut |part| if let Node::Symbol(name) = part {
		if name.starts_with(SYSTEM_PREFIX) {
			names.insert(name.clone());
		}
	});
	names
}

fn first_word(statement: &Node) -> String {
	match statement.drop_meta() {
		Node::List(items, _, _) => items.first().map(word).unwrap_or_default(),
		_ => String::new(),
	}
}

/// The names the program binds: assigned, defined, parameters of functions and lambdas
fn bound_names(program: &Node) -> HashSet<String> {
	let mut bound = HashSet::new();
	program.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::Define | Op::Arrow | Op::FatArrow, _) = part {
		bound.extend(crate::variable_signals::symbols(target));
	});
	bound
}

/// Each read of a system value as its mark; a field name (`{battery: 5}`, `p.battery`) is no read
fn mark(node: Node, names: &[(&str, bool)]) -> Node {
	let marked = |name: &str| names.iter().any(|(system, _)| *system == name).then(|| Node::Symbol(format!("{SYSTEM_PREFIX}{name}")));
	match node {
		Node::Symbol(name) => marked(&name).unwrap_or(Node::Symbol(name)),
		Node::Key(left, op @ (Op::Colon | Op::Dot), right) => {
			let right = if op == Op::Dot { *right } else { mark(*right, names) };
			Node::Key(left, op, Box::new(right))
		}
		Node::List(items, bracket, separator) => {
			// `dark mode`: a phrase of two words
			let (mut joined, mut phrases) = (vec![], 0);
			let mut items = items.into_iter().peekable();
			while let Some(item) = items.next() {
				let phrase = items.peek().map(|next| format!("{} {}", word(&item), word(next))).and_then(|phrase| marked(&phrase));
				match phrase {
					Some(phrase) => {
						items.next();
						phrases += 1;
						joined.push(phrase);
					}
					None => joined.push(mark(item, names)),
				}
			}
			match (joined.len(), &bracket) {
				(1, Bracket::None) if phrases > 0 => joined.pop().expect("one item"),
				_ => Node::List(joined, bracket, separator),
			}
		}
		other => other.map_children(|child| mark(child, names)),
	}
}

/// `whenever battery < 20% {…}`, which arrives as `whenever battery < (20 % {…})`: `whenever battery < 20 {…}`
fn with_percent_body(statement: Node) -> Node {
	let Node::List(items, bracket, separator) = statement.drop_meta().clone() else { return statement };
	let [listener, condition] = items.as_slice() else { return statement };
	if !marked_names(condition).contains(&format!("{SYSTEM_PREFIX}{BATTERY}")) {
		return statement;
	}
	match percent_body(condition) {
		Some((condition, body)) => Node::List(vec![listener.clone(), condition, body], bracket, separator),
		None => statement,
	}
}

fn percent_body(condition: &Node) -> Option<(Node, Node)> {
	let Node::Key(left, op, right) = condition.drop_meta() else { return None };
	match right.drop_meta() {
		Node::Key(percent, Op::Mod, body) if matches!(body.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			Some((Node::Key(left.clone(), *op, percent.clone()), body.as_ref().clone()))
		}
		_ => percent_body(right).map(|(right, body)| (Node::Key(left.clone(), *op, Box::new(right)), body)),
	}
}

/// Each mark as the host call
fn reading(node: Node) -> Node {
	match node {
		Node::Symbol(name) if name.starts_with(SYSTEM_PREFIX) => {
			let name = name[SYSTEM_PREFIX.len()..].to_string();
			let yes_no = SYSTEM_VALUES.iter().any(|(system, yes_no)| *system == name && *yes_no);
			let call = Node::List(vec![Node::Symbol(SYSTEM_VALUE.to_string()), Node::Text(name)], Bracket::Round, Separator::None);
			if yes_no { Node::Key(Box::new(call), Op::Ne, Box::new(Node::int(0))) } else { call }
		}
		other => other.map_children(reading),
	}
}
