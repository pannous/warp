//! `go { … }` starts a block as a task (user, 2026-10-06: `go { hi() }; print('faster')` prints faster first). The
//! block becomes a function of the variables it reads that are assigned before it, `go·block·1(n) := { … }`, started
//! like any other function, `go go·block·1(n)`: the task gets their values as they are at its start, copied
//! (notes/go_blocks.md). declarations::lower_tasks then decides how the function runs.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const GO_WORD: &str = "go";
const BLOCK_FUNCTION_PREFIX: &str = "go·block·";

pub fn lower(node: Node) -> Node {
	if !has_go_block(&node) || defines_go(&node) {
		return node;
	}
	let blocks = GoBlocks::default();
	let node = blocks.lower(node, &[]);
	crate::declarations::with_definitions_first(node, blocks.definitions.into_inner())
}

#[derive(Default)]
struct GoBlocks {
	definitions: std::cell::RefCell<Vec<Node>>,
}

impl GoBlocks {
	/// `known`: the variables assigned before this point (and the parameters of the enclosing function)
	fn lower(&self, node: Node, known: &[String]) -> Node {
		match node {
			Node::List(items, bracket, separator) if crate::variable_signals::is_statement_list(&bracket, &separator) => {
				let mut known = known.to_vec();
				let items = items.into_iter().map(|item| {
					let lowered = self.lower(item.clone(), &known);
					known.extend(assigned_variables(&item));
					lowered
				}).collect();
				Node::List(items, bracket, separator)
			}
			Node::List(items, bracket, separator) if started_block(&items).is_some() => {
				let block = self.lower(started_block(&items).expect("guarded").clone(), known);
				self.start(block, known, bracket, separator)
			}
			Node::Key(head, Op::Define, body) if matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)) => {
				let parameters: Vec<String> = match head.drop_meta() {
					Node::List(items, _, _) => items[1..].iter().map(parameter_name).collect(),
					_ => unreachable!("guarded"),
				};
				Node::Key(head, Op::Define, Box::new(self.lower(*body, &[known.to_vec(), parameters].concat())))
			}
			other => other.map_children(|child| self.lower(child, known)),
		}
	}

	/// The block as a function of the known variables it reads, and its start
	fn start(&self, block: Node, known: &[String], bracket: Bracket, separator: Separator) -> Node {
		let mut definitions = self.definitions.borrow_mut();
		let name = Node::Symbol(format!("{BLOCK_FUNCTION_PREFIX}{}", definitions.len() + 1));
		let parameters: Vec<Node> = read_symbols(&block).into_iter().filter(|symbol| known.contains(symbol)).map(Node::Symbol).collect();
		let head = Node::List([vec![name.clone()], parameters.clone()].concat(), Bracket::Round, Separator::None);
		definitions.push(Node::Key(Box::new(head.clone()), Op::Define, Box::new(block)));
		let call = Node::List([vec![name], parameters].concat(), Bracket::Round, Separator::None);
		Node::List(vec![Node::Symbol(GO_WORD.to_string()), call], bracket, separator)
	}
}

/// `go { … }`: the block
fn started_block(items: &[Node]) -> Option<&Node> {
	match items {
		[go, block] if go.drop_meta().name() == GO_WORD && matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) => Some(block),
		_ => None,
	}
}

fn has_go_block(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::List(items, _, _) if started_block(items).is_some()));
	found
}

/// A program defining its own `go` keeps it
fn defines_go(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| if let Node::Key(head, Op::Define | Op::Assign, _) = part {
		found |= match head.drop_meta() {
			Node::List(items, Bracket::Round, _) => items.first().is_some_and(|name| name.drop_meta().name() == GO_WORD),
			other => other.name() == GO_WORD,
		};
	});
	found
}

fn assigned_variables(node: &Node) -> Vec<String> {
	let mut names = vec![];
	node.visit(&mut |part| if let Node::Key(target, Op::Assign | Op::Define, _) = part {
		if let Node::Symbol(name) = target.drop_meta() {
			names.push(name.clone());
		}
	});
	names
}

/// The symbols of a block in their first order, each once
fn read_symbols(node: &Node) -> Vec<String> {
	let mut symbols: Vec<String> = vec![];
	node.visit(&mut |part| if let Node::Symbol(name) = part {
		if !symbols.contains(name) {
			symbols.push(name.clone());
		}
	});
	symbols
}

/// `x`, `x: int`, `x = 1`: the parameter's name
fn parameter_name(parameter: &Node) -> String {
	match parameter.drop_meta() {
		Node::Key(name, _, _) => name.drop_meta().name(),
		other => other.name(),
	}
}
