//! `go { … }` starts a block as a task (user, 2026-10-06: `go { hi() }; print('faster')` prints faster first). The
//! block becomes a function of the variables it reads that are assigned before it, `go·block·1(n) := { … }`, started
//! like any other function, `go go·block·1(n)`: the task gets their values as they are at its start, copied
//! (notes/go_blocks.md). declarations::lower_tasks then decides how the function runs.
//! `after done return x` (wiki/thread.md) is the go block `go { while not done { sleep(1) }; x }`, its condition read
//! from shared values (P106); `await job or y` is `try await job else y`.
//! `go xs.map(f)`, `go for x in xs {…}` and `xs.map(f) @parallel` split their items into tasks (parallel.rs).

use super::words::{FOR_WORD, RETURN_WORD};
use super::nodes::{call, key};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const GO_WORD: &str = "go";
const BLOCK_FUNCTION_PREFIX: &str = "go·block·";
/// A go block's parameter is the variable it reads with this suffix: `n` comes in as `n·in`
const INPUT_SUFFIX: &str = "·in";
const AFTER_WORD: &str = "after";
const AWAIT_WORD: &str = "await";
const AFTER_CONDITION: &str = "after_condition_placeholder";
const AFTER_VALUE: &str = "after_value_placeholder";
/// How often a waiting `after` checks its condition, in milliseconds
const AFTER_POLL_MILLISECONDS: i64 = 1;

pub fn lower(node: Node) -> Node {
	if defines_go(&node) {
		return node;
	}
	let phrases = Phrases { assigned: assigned_variables(&node), shared: crate::shared_arrays::shared_names(&node), parallel_maps: Default::default() };
	let node = task_phrases(node, &phrases);
	if !has_go_block(&node) {
		return node;
	}
	let blocks = GoBlocks::default();
	let node = blocks.lower(node, &[]);
	crate::declarations::with_definitions_first(node, blocks.definitions.into_inner())
}

/// A function as a message names it: `go·block·1` is `go block 1`
pub fn written_name(function: &str) -> String {
	match function.strip_prefix(BLOCK_FUNCTION_PREFIX) {
		Some(number) => format!("{GO_WORD} block {number}"),
		None => function.to_string(),
	}
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
			Node::List(items, bracket, separator) if !separator.separates_statements() && starts_block_at(&items).is_some() => {
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
		let inputs: Vec<String> = read_symbols(&block).into_iter().filter(|symbol| known.contains(symbol)).collect();
		// each input under a name of its own: a parameter named like a function of the program (`inc = x => x + 1`)
		// would read as that function
		let parameter = |input: &String| Node::Symbol(format!("{input}{INPUT_SUFFIX}"));
		let block = inputs.iter().fold(block, |block, input| crate::library_words::substitute(block, input, &parameter(input)));
		let head = Node::List([vec![name.clone()], inputs.iter().map(parameter).collect()].concat(), Bracket::Round, Separator::None);
		definitions.push(key(head, Op::Define, block));
		Node::List([vec![name], inputs.into_iter().map(Node::Symbol).collect()].concat(), Bracket::Round, Separator::None)
	}
}

struct Phrases {
	/// the program's variables, and those of them that every task shares
	assigned: Vec<String>,
	shared: Vec<String>,
	/// how many parallel maps and loops got their names
	parallel_maps: std::cell::Cell<usize>,
}

impl Phrases {
	/// The number of the next parallel map or loop, naming its temporaries apart
	fn next_parallel(&self) -> usize {
		self.parallel_maps.replace(self.parallel_maps.get() + 1)
	}
}

/// `after C return V` → `go { while not (C) { sleep(1) }; V }`, `await job or y` → `try await job else y`
fn task_phrases(node: Node, phrases: &Phrases) -> Node {
	if let Some((list, function)) = crate::parallel::parallel_map(&node) {
		return crate::parallel::map_in_tasks(task_phrases(list, phrases), task_phrases(function, phrases), phrases.next_parallel());
	}
	if let Some(error) = crate::parallel::sequential_warning(&node) {
		return error;
	}
	match node {
		Node::List(items, bracket, separator) => {
			let items: Vec<Node> = items.into_iter().map(|item| task_phrases(item, phrases)).collect();
			if let Some((variable, list, body)) = crate::parallel::parallel_loop(&items) {
				return crate::parallel::loop_in_tasks(variable, list, body, phrases.next_parallel(), &phrases.shared);
			}
			match after_parts(&items) {
				Some((condition, value)) => after_task(condition, value, phrases),
				None => Node::List(items, bracket, separator),
			}
		}
		Node::Key(awaited, Op::Or, fallback) if matches!(awaited.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|word| word.drop_meta().name() == AWAIT_WORD)) => {
			let guarded = call(crate::warp_parser::TRY_MARKER, vec![*awaited, task_phrases(*fallback, phrases)]);
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

/// `after C return V`: C and V, from the parser's `after·return(C, V)` or written flat or in spaced groups
fn after_parts(items: &[Node]) -> Option<(Node, Node)> {
	if let [marker, condition, value] = items {
		if marker.drop_meta().name() == crate::warp_parser::AFTER_MARKER {
			return Some((condition.clone(), value.clone()));
		}
	}
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
	let template = crate::warp_parser::parse(&format!("{GO_WORD} {{ while not ({AFTER_CONDITION}) {{ sleep({AFTER_POLL_MILLISECONDS} ms) }}; {AFTER_VALUE} }}"));
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
