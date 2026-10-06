//! `go { … }` starts a block as a task (user, 2026-10-06: `go { hi() }; print('faster')` prints faster first). The
//! block becomes a function of the variables it reads that are assigned before it, `go·block·1(n) := { … }`, started
//! like any other function, `go go·block·1(n)`: the task gets their values as they are at its start, copied
//! (notes/go_blocks.md). declarations::lower_tasks then decides how the function runs.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const GO_WORD: &str = "go";
const BLOCK_FUNCTION_PREFIX: &str = "go·block·";
const FOR_WORD: &str = "for";

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
			// `go { … }`, also inside a call: `jobs.add(go { … })` is `add go {…}`
			Node::List(items, bracket, separator) if !is_sequence(&separator) && starts_block_at(&items).is_some() => {
				let at = starts_block_at(&items).expect("guarded");
				let mut items: Vec<Node> = items.into_iter().map(|item| self.lower(item, known)).collect();
				items[at] = self.function_of(items[at].clone(), known);
				Node::List(items, bracket, separator)
			}
			Node::List(items, bracket, separator) if crate::variable_signals::is_statement_list(&bracket, &separator) => {
				let mut known = known.to_vec();
				let items = items.into_iter().map(|item| {
					let lowered = self.lower(item.clone(), &known);
					known.extend(assigned_variables(&item));
					lowered
				}).collect();
				Node::List(items, bracket, separator)
			}
			// `for i in …`: the loop variable
			Node::List(items, bracket, separator) if items.len() > 2 && items[0].drop_meta().name() == FOR_WORD => {
				let known = [known.to_vec(), vec![items[1].drop_meta().name()]].concat();
				Node::List(items.into_iter().map(|item| self.lower(item, &known)).collect(), bracket, separator)
			}
			// `f(x) := …`: the parameters
			Node::Key(head, Op::Define, body) if matches!(head.drop_meta(), Node::List(_, Bracket::Round, _)) => {
				let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
				let parameters: Vec<String> = items[1..].iter().map(parameter_name).collect();
				Node::Key(head, Op::Define, Box::new(self.lower(*body, &[known.to_vec(), parameters].concat())))
			}
			// `x => …`, `(x, y) => …`
			Node::Key(parameters, Op::FatArrow, body) => {
				let names: Vec<String> = match parameters.drop_meta() {
					Node::List(items, _, _) => items.iter().map(parameter_name).collect(),
					single => vec![parameter_name(single)],
				};
				Node::Key(parameters, Op::FatArrow, Box::new(self.lower(*body, &[known.to_vec(), names].concat())))
			}
			other => other.map_children(|child| self.lower(child, known)),
		}
	}

	/// The block as a function of the known variables it reads: the call that starts it
	fn function_of(&self, block: Node, known: &[String]) -> Node {
		let mut definitions = self.definitions.borrow_mut();
		let name = Node::Symbol(format!("{BLOCK_FUNCTION_PREFIX}{}", definitions.len() + 1));
		let parameters: Vec<Node> = read_symbols(&block).into_iter().filter(|symbol| known.contains(symbol)).map(Node::Symbol).collect();
		let head = Node::List([vec![name.clone()], parameters.clone()].concat(), Bracket::Round, Separator::None);
		definitions.push(Node::Key(Box::new(head.clone()), Op::Define, Box::new(block)));
		Node::List([vec![name], parameters].concat(), Bracket::Round, Separator::None)
	}
}

fn is_sequence(separator: &Separator) -> bool {
	matches!(separator, Separator::Semicolon | Separator::Newline)
}

/// `go { … }` among the items of a list: the index of the block
fn starts_block_at(items: &[Node]) -> Option<usize> {
	items.windows(2).position(|pair| pair[0].drop_meta().name() == GO_WORD && matches!(pair[1].drop_meta(), Node::List(_, Bracket::Curly, _)))
		.map(|go| go + 1)
}

fn has_go_block(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::List(items, _, _) if starts_block_at(items).is_some()));
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
