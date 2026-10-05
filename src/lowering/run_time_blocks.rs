//! Blocks that can never run (notes/runtime_eval.md step 0, user 2026-10-05: "give a compiler warning if the code
//! contains unresolved symbol arithmetic"): `x : a+b` where the program defines a and b nowhere. Names inside a block
//! resolve where `!` runs it (wiki/charged.md §5), so a name that appears nowhere else in the program can never resolve:
//! the block is a typo, or symbolic data that wants `data a+b`.
//! A block known only at run time runs through the host (run_block, notes/runtime_eval.md step 2).

use crate::diagnostic::{ask, reading, Ask, Fallback};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;

const UNRESOLVED_TOPIC: &str = "unresolved-block";
/// The spec's word for `x!!`: run the block fully
const INTERPRET: &str = "interpret";

pub fn warn_unresolved(program: Node) -> Node {
	let mut warnings: Vec<Ask> = vec![];
	visit_statements(&program, &mut |statement| {
		let Some((name, block)) = crate::blocks::uncharged(statement) else { return };
		let unresolved: Vec<String> = arithmetic_operands(&block).into_iter()
			.filter(|operand| occurrences(&program, operand) == occurrences(&block, operand))
			.collect();
		if !unresolved.is_empty() {
			warnings.push(unresolved_warning(&name, &block, &unresolved).at_node(statement));
		}
	});
	for warning in warnings {
		if let Err(error) = ask(&warning) {
			return error;
		}
	}
	program
}

fn unresolved_warning(name: &str, block: &Node, unresolved: &[String]) -> Ask {
	let written = crate::normalize::operand_text(block);
	let names = unresolved.join(", ");
	let question = format!("{name} keeps {written} over {names}, defined nowhere in the program: {name}! can never run it");
	let readings = vec![reading("keeping the block", &format!("{name} = data {written}"))];
	Ask::new(UNRESOLVED_TOPIC, question, readings, Fallback::Warning).written(&format!("{name} : {written}"))
}

/// Every statement of the program and of its nested statement lists
fn visit_statements(program: &Node, visit: &mut dyn FnMut(&Node)) {
	match program.drop_meta() {
		Node::List(items, _, Separator::Semicolon | Separator::Newline) => items.iter().for_each(|item| visit_statements(item, visit)),
		Node::Key(_, _, right) => {
			visit(program);
			visit_statements(right, visit);
		}
		other => visit(other),
	}
}

/// The names an arithmetic or comparison operator of the block applies to, each once
fn arithmetic_operands(block: &Node) -> Vec<String> {
	let mut names: Vec<String> = vec![];
	block.visit(&mut |node| {
		if let Node::Key(left, op, right) = node {
			if op.is_arithmetic() || op.is_comparison() {
				for operand in [left, right] {
					if let Node::Symbol(name) = operand.drop_meta() {
						if !names.contains(name) {
							names.push(name.clone());
						}
					}
				}
			}
		}
	});
	names.sort();
	names
}

fn occurrences(node: &Node, name: &str) -> usize {
	let mut count = 0;
	node.visit(&mut |part| count += matches!(part, Node::Symbol(word) if word == name) as usize);
	count
}

/// `interpret e` (wiki/charged.md §5: `x!!` or `interpret x`): of a block bound by `x : e` it is `x!!`, which blocks.rs
/// runs where it is written; of any other value it runs the block at run time through the host,
/// `run_block(e, "a b", [a, b], data [definitions])`: the main-level variables assigned before it (a snapshot the
/// block reads) and the program's function definitions (the block may call them)
pub fn lower_interpret(program: Node) -> Node {
	let statements = match program.drop_meta() {
		Node::List(items, Bracket::None, Separator::Semicolon | Separator::Newline) => items.clone(),
		_ => vec![program.clone()],
	};
	if !statements.iter().any(mentions_interpret) {
		return program;
	}
	let mut site = Site { blocks: vec![], variables: vec![], definitions: statements.iter().filter(|statement| defines_functions(statement)).cloned().collect() };
	let mut lowered = vec![];
	for statement in statements {
		lowered.push(site.interpreted(statement.clone()));
		site.follow(&statement);
	}
	match program {
		Node::List(_, bracket, separator) => Node::List(lowered, bracket, separator),
		Node::Meta { data, .. } => Node::Meta { node: Box::new(Node::List(lowered, Bracket::None, Separator::Newline)), data },
		_ => lowered.into_iter().next().unwrap_or(Node::Empty),
	}
}

/// What a run-time block sees where it runs
struct Site {
	/// the constant blocks bound so far (`x : e`)
	blocks: Vec<String>,
	/// the main-level variables assigned so far
	variables: Vec<String>,
	/// the program's function definitions
	definitions: Vec<Node>,
}

fn defines_functions(statement: &Node) -> bool {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, statement);
	!context.user_functions.is_empty() && crate::blocks::uncharged(statement).is_none()
}

fn mentions_interpret(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| found |= matches!(part, Node::Symbol(word) if word == INTERPRET));
	found
}

/// `interpret e` / `interpret(e)`: e
fn interpret_argument(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::List(items, _, _) if items.len() == 2 && matches!(items[0].drop_meta(), Node::Symbol(word) if word == INTERPRET) => Some(&items[1]),
		_ => None,
	}
}

fn as_data(node: Node) -> Node {
	Node::List(vec![Node::Symbol(crate::blocks::DATA_WORD.to_string()), node], Bracket::None, Separator::Space)
}

impl Site {
	/// After a statement: the block or variable it binds
	fn follow(&mut self, statement: &Node) {
		if let Some((name, _)) = crate::blocks::uncharged(statement) {
			self.blocks.push(name);
			return;
		}
		if let Node::Key(target, Op::Assign, _) = statement.drop_meta() {
			if let Node::Symbol(name) = target.drop_meta() {
				self.blocks.retain(|block| block != name);
				if !self.variables.contains(name) {
					self.variables.push(name.clone());
				}
			}
		}
	}

	fn interpreted(&self, node: Node) -> Node {
		if let Some(argument) = interpret_argument(&node) {
			let argument = self.interpreted(argument.clone());
			return match argument.drop_meta() {
				Node::Symbol(name) if self.blocks.contains(name) => crate::mutation::marked_fully(argument),
				_ => {
					let names = Node::Text(self.variables.join(" "));
					let values = Node::List(self.variables.iter().map(|name| Node::Symbol(name.clone())).collect(), Bracket::Square, Separator::Colon);
					let definitions = as_data(Node::List(self.definitions.clone(), Bracket::Square, Separator::Colon));
					Node::List(vec![Node::Symbol(crate::host::RUN_BLOCK.to_string()), argument, names, values, definitions], Bracket::Round, Separator::None)
				}
			};
		}
		match node {
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.interpreted(item)).collect(), bracket, separator),
			Node::Key(left, op, right) => Node::Key(Box::new(self.interpreted(*left)), op, Box::new(self.interpreted(*right))),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.interpreted(*node)), data },
			other => other,
		}
	}
}
