//! Channels inside one run (P155, notes/channels.md): Go's unbuffered channel on the host words of tasks.rs.
//! `ch = channel()` is `ch = channel_new()`, `ch.send(v)` and `send v to ch` are `channel_put(ch, v)` (it waits until a
//! receiver took v), `ch.receive()` is `channel_take(ch)`, `ch.close()` is `channel_close(ch)`, and `for v in ch {…}`
//! receives until the channel is closed: `while channel_more(ch) { v = channel_take(ch); … }`. The channels are the
//! variables assigned `channel()` and the parameters of the functions called with one. First of the source passes:
//! go_blocks renames what a go block reads, system_signals takes `send v to "chat"` for the machine channel.
//! `chat = channel "chat"` is the machine-wide channel (src/channels.rs) with the same words: it listens from its
//! assignment on, `chat.send(v)` and `send v to chat` are `channel_send(chat, v)`, `chat.receive()` waits for a message.

use super::nodes::{call, key};
use crate::declarations::word;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use std::collections::{HashMap, HashSet};

const CHANNEL_WORD: &str = "channel";
const SEND_WORD: &str = "send";
const RECEIVE_WORD: &str = "receive";
const CLOSE_WORD: &str = "close";
const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";
const NEW: &str = crate::host::CHANNEL_WORDS[0];
const PUT: &str = crate::host::CHANNEL_WORDS[1];
const TAKE: &str = crate::host::CHANNEL_WORDS[2];
const CLOSE: &str = crate::host::CHANNEL_WORDS[4];
const FOR_TEMPLATE: &str = "while channel_more(CHANNEL) { ITEM = channel_take(CHANNEL); BODY }";
const RECEIVE_TEMPLATE: &str = "do { while channel_pending(ID) == 0 { sleep(CHECK ms) }; channel_next(ID) }";
/// How often a machine channel's receive looks for a message, in milliseconds
const RECEIVE_CHECK_MILLISECONDS: i64 = 5;
/// The listener ids of machine channel variables, apart from those system_signals counts from 0
const MACHINE_LISTENER_BASE: i64 = 1 << 20;

pub fn lower(program: Node) -> Node {
	let local = channel_names(&program);
	let machine = machine_channels(&program);
	if local.is_empty() && machine.is_empty() {
		return program;
	}
	lowered(program, &local, &machine)
}

/// The variables assigned `channel "chat"` and their listener ids
fn machine_channels(program: &Node) -> HashMap<String, i64> {
	let mut names = vec![];
	program.visit(&mut |part| if let Node::Key(target, Op::Assign, value) = part {
		if machine_channel(value).is_some() && !names.contains(&word(target)) {
			names.push(word(target));
		}
	});
	names.into_iter().zip(MACHINE_LISTENER_BASE..).collect()
}

/// `channel "chat"`: the name
fn machine_channel(node: &Node) -> Option<Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::None, _) if items.len() == 2 && word(&items[0]) == CHANNEL_WORD => match items[1].drop_meta() {
			Node::Text(_) => Some(items[1].clone()),
			Node::Char(letter) => Some(Node::Text(letter.to_string())),
			_ => None,
		},
		_ => None,
	}
}

/// Statements where `chat = channel "chat"` is `chat = "chat"; channel_listen(ID, "chat")`
fn with_listeners(node: Node, machine: &HashMap<String, i64>) -> Node {
	fn listened(statement: &Node, machine: &HashMap<String, i64>) -> Option<(Node, i64, Node)> {
		let Node::Key(target, Op::Assign, value) = statement.drop_meta() else { return None };
		Some((machine_channel(value)?, *machine.get(&word(target))?, target.as_ref().clone()))
	}
	let Node::List(items, bracket, separator @ (Separator::Semicolon | Separator::Newline)) = node.drop_meta() else { return node };
	if !items.iter().any(|item| listened(item, machine).is_some()) {
		return node;
	}
	let items = items.iter().flat_map(|item| match listened(item, machine) {
		Some((name, id, target)) => vec![key(target, Op::Assign, name.clone()), call(crate::host::CHANNEL_LISTEN, vec![Node::int(id), name])],
		None => vec![item.clone()],
	});
	Node::List(items.collect(), bracket.clone(), separator.clone())
}

fn filled(template: &str, bindings: &[(&str, Node)]) -> Node {
	let bindings: HashMap<String, Node> = bindings.iter().map(|(name, node)| (name.to_string(), node.clone())).collect();
	crate::law::substitute(crate::warp_parser::parse(template).drop_meta(), &bindings)
}

/// The variables assigned `channel()`, then the parameters of the functions called with a channel, until none is new
fn channel_names(program: &Node) -> HashSet<String> {
	let mut channels = HashSet::new();
	let mut functions: HashMap<String, Vec<String>> = HashMap::new();
	program.visit(&mut |part| match part {
		Node::Key(target, Op::Assign, value) if is_new_channel(value) => { channels.insert(word(target)); }
		Node::Key(head, Op::Define, _) => if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			if let Some((name, parameters)) = items.split_first() {
				functions.insert(word(name), parameters.iter().map(word).collect());
			}
		},
		_ => {}
	});
	if channels.is_empty() || functions.is_empty() {
		return channels;
	}
	loop {
		let mut found = vec![];
		program.visit(&mut |part| if let Node::List(items, Bracket::Round, _) = part {
			let Some((callee, arguments)) = items.split_first() else { return };
			let Some(parameters) = functions.get(&word(callee)) else { return };
			let given = arguments.iter().zip(parameters).filter(|(argument, _)| channels.contains(&word(argument)));
			found.extend(given.map(|(_, parameter)| parameter.clone()).filter(|parameter| !channels.contains(parameter)));
		});
		if found.is_empty() {
			return channels;
		}
		channels.extend(found);
	}
}

/// `channel()`
fn is_new_channel(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() == 1 && word(&items[0]) == CHANNEL_WORD)
}

fn lowered(node: Node, local: &HashSet<String>, machine: &HashMap<String, i64>) -> Node {
	let node = with_listeners(node.map_children(|child| lowered(child, local, machine)), machine);
	if is_new_channel(&node) {
		return call(NEW, vec![]);
	}
	let is_local = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if local.contains(name));
	let listener = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) => machine.get(name).map(|id| Node::int(*id)),
		_ => None,
	};
	let sent = |channel: &Node, value: &Node| match listener(channel) {
		Some(_) => Some(call(crate::host::CHANNEL_SEND, vec![channel.clone(), value.clone()])),
		None => is_local(channel).then(|| call(PUT, vec![channel.clone(), value.clone()])),
	};
	match node.drop_meta() {
		Node::Key(channel, Op::Dot, method) => {
			let Node::List(items, _, _) = method.drop_meta() else { return node };
			let lowered = match (items.first().map(word).as_deref(), &items[1..]) {
				(Some(SEND_WORD), [value]) => sent(channel, value),
				(Some(RECEIVE_WORD), []) => match listener(channel) {
					Some(id) => Some(filled(RECEIVE_TEMPLATE, &[("ID", id), ("CHECK", Node::int(RECEIVE_CHECK_MILLISECONDS))])),
					None => is_local(channel).then(|| call(TAKE, vec![channel.as_ref().clone()])),
				},
				(Some(CLOSE_WORD), []) if is_local(channel) => Some(call(CLOSE, vec![channel.as_ref().clone()])),
				_ => None,
			};
			lowered.unwrap_or(node)
		}
		Node::List(items, bracket, _) => match items.as_slice() {
			[send, value_to] if word(send) == SEND_WORD => match value_to.drop_meta() {
				// `{ send 7 to ch }` is a block of one statement: it stays a block
				Node::Key(value, Op::To, channel) => match sent(channel, value) {
					Some(put) if *bracket == Bracket::Curly => Node::List(vec![put], Bracket::Curly, Separator::Semicolon),
					Some(put) => put,
					None => node,
				},
				_ => node,
			},
			[for_word, item, in_word, channel, body] if word(for_word) == FOR_WORD && word(in_word) == IN_WORD && is_local(channel) => {
				filled(FOR_TEMPLATE, &[("CHANNEL", channel.clone()), ("ITEM", item.clone()), ("BODY", body.clone())])
			}
			_ => node,
		},
		_ => node,
	}
}

