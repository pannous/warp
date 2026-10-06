//! `go { … }` starts a block as a task (user, 2026-10-06: `go { hi() }; print('faster')` prints faster first). The
//! block becomes a function of the variables it reads that are assigned before it, `go·block·1(n) := { … }`, started
//! like any other function, `go go·block·1(n)`: the task gets their values as they are at its start, copied
//! (notes/go_blocks.md). declarations::lower_tasks then decides how the function runs.
//! `after done return x` (wiki/thread.md) is the go block `go { while not done { sleep(1) }; x }`, its condition read
//! from shared values (P106); `await job or y` is `try await job else y`.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const GO_WORD: &str = "go";
const BLOCK_FUNCTION_PREFIX: &str = "go·block·";
const FOR_WORD: &str = "for";
const AFTER_WORD: &str = "after";
const RETURN_WORD: &str = "return";
const AWAIT_WORD: &str = "await";
const AFTER_CONDITION: &str = "after_condition_placeholder";
const AFTER_VALUE: &str = "after_value_placeholder";
/// How often a waiting `after` checks its condition, in milliseconds
const AFTER_POLL_MILLISECONDS: i64 = 1;

pub fn lower(node: Node) -> Node {
	if defines_go(&node) {
		return node;
	}
	let node = task_phrases(node.clone(), &Phrases { assigned: assigned_variables(&node), shared: crate::shared_arrays::shared_names(&node) });
	if !has_go_block(&node) {
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

struct Phrases {
	/// the program's variables, and those of them that every task shares
	assigned: Vec<String>,
	shared: Vec<String>,
}

/// `after C return V` → `go { while not (C) { sleep(1) }; V }`, `await job or y` → `try await job else y`
fn task_phrases(node: Node, phrases: &Phrases) -> Node {
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| task_phrases(item, phrases)).collect();
			match after_parts(&items) {
				Some((condition, value)) => after_task(condition, value, phrases),
				None => Node::List(items, bracket, separator),
			}
		}
		Node::Key(awaited, Op::Or, fallback) if matches!(awaited.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.drop_meta().name() == AWAIT_WORD)) => {
			let guarded = Node::List(vec![Node::Symbol(crate::wasp_parser::TRY_MARKER.to_string()), *awaited, task_phrases(*fallback, phrases)], Bracket::Round, Separator::None);
			task_phrases_inside(guarded, phrases)
		}
		other => other.map_children(|child| task_phrases(child, phrases)),
	}
}

/// The awaited task itself may hold phrases: `await (after …) or y`
fn task_phrases_inside(guarded: Node, phrases: &Phrases) -> Node {
	match guarded {
		Node::List(mut items, bracket, separator) => {
			items[1] = task_phrases(items[1].clone(), phrases);
			Node::List(items, bracket, separator)
		}
		other => other,
	}
}

/// `after C return V`, written flat or in spaced groups: C and V
fn after_parts(items: &[Node]) -> Option<(Node, Node)> {
	let words: Vec<Node> = items.iter().flat_map(|item| match item.drop_meta() {
		Node::List(inner, Bracket::None, Separator::Space) => inner.clone(),
		other => vec![other.clone()],
	}).collect();
	if words.first()?.drop_meta().name() != AFTER_WORD {
		return None;
	}
	let at = words.iter().position(|word| word.drop_meta().name() == RETURN_WORD)?;
	let group = |part: &[Node]| match part {
		[single] => Some(single.clone()),
		[] => None,
		several => Some(Node::List(several.to_vec(), Bracket::None, Separator::Space)),
	};
	Some((group(&words[1..at])?, group(&words[at + 1..])?))
}

fn after_task(condition: Node, value: Node, phrases: &Phrases) -> Node {
	let copied: Vec<String> = read_symbols(&condition).into_iter().filter(|name| phrases.assigned.contains(name) && !phrases.shared.contains(name)).collect();
	if let Some(name) = copied.first() {
		return crate::node::error(&format!("after {name}: a task gets a copy of {name}, which never changes there; share it: `shared {name} = …` (P106)"));
	}
	let template = crate::wasp_parser::parse(&format!("{GO_WORD} {{ while not ({AFTER_CONDITION}) {{ sleep({AFTER_POLL_MILLISECONDS}) }}; {AFTER_VALUE} }}"));
	let template = crate::library_words::substitute(template, AFTER_CONDITION, &condition);
	crate::library_words::substitute(template, AFTER_VALUE, &value)
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
