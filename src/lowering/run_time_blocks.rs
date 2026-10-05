//! Blocks that can never run (notes/runtime_eval.md step 0, user 2026-10-05: "give a compiler warning if the code
//! contains unresolved symbol arithmetic"): `x : a+b` where the program defines a and b nowhere. Names inside a block
//! resolve where `!` runs it (wiki/charged.md §5), so a name that appears nowhere else in the program can never resolve:
//! the block is a typo, or symbolic data that wants `data a+b`.

use crate::diagnostic::{ask, reading, Ask, Fallback};
use crate::node::{Node, Separator};

const UNRESOLVED_TOPIC: &str = "unresolved-block";

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
