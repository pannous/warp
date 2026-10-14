//! `dir(time)`, `dir text` (Python's dir, card dir-introspection): the names a standard module defines, a list of
//! texts in the order the module defines them. `methods(math)`, `math.methods` (card module-methods, decision
//! reflection): its functions themselves, `function gcd` for each. A program's own `dir` or `methods`, or its own name
//! of the module, wins

use crate::node::{Bracket, Node};
use crate::operators::Op;
use crate::words::DIR_WORD;
use std::collections::HashSet;

const METHODS_WORD: &str = "methods";

pub fn lower(node: Node) -> Node {
	if !node.mentions_any(&[DIR_WORD, METHODS_WORD]) {
		return node;
	}
	let defined = crate::library_words::defined_names(&node);
	let words: Vec<&str> = [DIR_WORD, METHODS_WORD].into_iter().filter(|word| !defined.contains(*word)).collect();
	listed_names(node, &words, &defined)
}

fn listed_names(node: Node, words: &[&str], defined: &HashSet<String>) -> Node {
	let answer = match module_word(&node) {
		Some((word, module)) if words.contains(&word) => module_names(module, defined).map(|names| match word {
			DIR_WORD => crate::reflection::text_list(names),
			_ => crate::reflection::function_list(names),
		}),
		_ => None,
	};
	answer.unwrap_or_else(|| node.map_children(|child| listed_names(child, words, defined)))
}

/// `dir(math)`, `dir math`, `math.methods`: the word and the module
fn module_word(node: &Node) -> Option<(&str, &Node)> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round | Bracket::None, _) if items.len() == 2 => {
			Some((items[0].drop_meta().symbol_name()?, &items[1]))
		}
		Node::Key(module, Op::Dot, word) if word.symbol_name() == Some(METHODS_WORD) => Some((METHODS_WORD, module)),
		_ => None,
	}
}

/// The names of the standard module `module` names, unless the program names something so itself
fn module_names(module: &Node, defined: &std::collections::HashSet<String>) -> Option<&'static [String]> {
	let Node::Symbol(name) = module.drop_meta() else {
		return None;
	};
	if defined.contains(name) {
		return None;
	}
	crate::modules::std_module_definitions(name)
}
