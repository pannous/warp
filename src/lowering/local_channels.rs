//! Channels inside one run (P155, notes/channels.md): Go's unbuffered channel on the host words of tasks.rs.
//! `ch = channel()` is `ch = channel_new()`, `ch.send(v)` and `send v to ch` are `channel_put(ch, v)` (it waits until a
//! receiver took v), `ch.receive()` is `channel_take(ch)`, `ch.close()` is `channel_close(ch)`, and `for v in ch {…}`
//! receives until the channel is closed: `while channel_more(ch) { v = channel_take(ch); … }`. The channels are the
//! variables assigned `channel()` and the parameters of the functions called with one. First of the source passes:
//! go_blocks renames what a go block reads, system_signals takes `send v to "chat"` for the machine channel.

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

pub fn lower(program: Node) -> Node {
	let channels = channel_names(&program);
	if channels.is_empty() {
		return program;
	}
	lowered(program, &channels)
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

fn lowered(node: Node, channels: &HashSet<String>) -> Node {
	let node = node.map_children(|child| lowered(child, channels));
	if is_new_channel(&node) {
		return call(NEW, vec![]);
	}
	let is_channel = |node: &Node| matches!(node.drop_meta(), Node::Symbol(name) if channels.contains(name));
	match node.drop_meta() {
		Node::Key(channel, Op::Dot, method) if is_channel(channel) => {
			let Node::List(items, _, _) = method.drop_meta() else { return node };
			let channel = channel.as_ref().clone();
			match (items.first().map(word).as_deref(), &items[1..]) {
				(Some(SEND_WORD), [value]) => call(PUT, vec![channel, value.clone()]),
				(Some(RECEIVE_WORD), []) => call(TAKE, vec![channel]),
				(Some(CLOSE_WORD), []) => call(CLOSE, vec![channel]),
				_ => node,
			}
		}
		Node::List(items, bracket, _) => match items.as_slice() {
			[send, sent] if word(send) == SEND_WORD => match sent.drop_meta() {
				// `{ send 7 to ch }` is a block of one statement: it stays a block
				Node::Key(value, Op::To, channel) if is_channel(channel) => {
					let put = call(PUT, vec![channel.as_ref().clone(), value.as_ref().clone()]);
					if *bracket == Bracket::Curly { Node::List(vec![put], Bracket::Curly, Separator::Semicolon) } else { put }
				}
				_ => node,
			},
			[for_word, item, in_word, channel, body] if word(for_word) == FOR_WORD && word(in_word) == IN_WORD && is_channel(channel) => {
				let bindings = HashMap::from([("CHANNEL".to_string(), channel.clone()), ("ITEM".to_string(), item.clone()), ("BODY".to_string(), body.clone())]);
				crate::law::substitute(crate::wasp_parser::parse(FOR_TEMPLATE).drop_meta(), &bindings)
			}
			_ => node,
		},
		_ => node,
	}
}

fn call(function: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(function.to_string())], arguments].concat(), Bracket::Round, Separator::None)
}
