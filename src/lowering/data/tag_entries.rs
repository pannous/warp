//! `name = value` inside a data block is the entry `name: value` (decisions.md tag-assign, markup-plain-library): the
//! block of a tag, `word{…}` or `word: {…}` where the word is no keyword and no function the program defines, holding no
//! statement besides its entries. So `a{ href="/about" "About" }`, `Person{ class = "a" }` and `cat{ x = 1 "t" }` hold
//! entries, while `kill{ hp = 0; print hp }` stays code. An element's block is always data, its loops and handlers
//! give children; HTML's own `label(for="pwd")` and the statement `p {class="note"}` take the same rule (P188). Before
//! soft_keywords, which would read `class = …` as a definition.
use crate::lowering::nodes::{function_head, DefinitionForm};
use crate::lowering::soft_keywords::{HARD_KEYWORDS, HIGHLIGHTED_WORDS};
use crate::node::{Bracket, Node};
use crate::operators::Op;
use std::collections::HashSet;

pub fn lower(program: Node) -> Node {
	let functions = defined_functions(&program);
	with_entries(program, &functions)
}

fn with_entries(node: Node, functions: &HashSet<String>) -> Node {
	let node = node.map_children(|child| with_entries(child, functions));
	match node {
		Node::Key(tag, Op::Colon, body) if is_data_tag(&tag, &body, functions) => {
			Node::Key(tag, Op::Colon, Box::new(entries_of(*body)))
		}
		// `label(for="pwd")`, `p {class="note"}`: an element's parentheses, or its block spaced from it
		Node::List(items, bracket, separator) if items.first().is_some_and(is_element_name) && items.len() > 1 => {
			let mut items = items.into_iter();
			let tag = items.next().into_iter();
			Node::List(tag.chain(items.map(entries_of)).collect(), bracket, separator)
		}
		other => other,
	}
}

/// `word{…}`: an element's block, or a block of no statement but entries after a word naming no function or keyword
fn is_data_tag(tag: &Node, body: &Node, functions: &HashSet<String>) -> bool {
	let Some(word) = tag.drop_meta().symbol_name() else {
		return false;
	};
	let Node::List(items, Bracket::Curly, _) = body.drop_meta() else {
		return false;
	};
	if is_element_name(tag) {
		return true;
	}
	let is_word = |words: &[&str]| words.contains(&word);
	let is_statement = |item: &Node| !is_entry(item) && crate::analyzer::is_statement(item, &Bracket::Curly);
	!is_word(&HARD_KEYWORDS)
		&& !is_word(&HIGHLIGHTED_WORDS)
		&& !functions.contains(word)
		&& !items.iter().any(is_statement)
}

fn is_element_name(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Symbol(tag) if crate::web::markup::is_element_tag(tag))
}

/// The items of a `{…}` or `(…)`, each `name = value` as `name: value`
fn entries_of(group: Node) -> Node {
	group.under_meta(|group| match group {
		Node::List(items, bracket @ (Bracket::Curly | Bracket::Round), separator) => {
			Node::List(items.into_iter().map(as_key).collect(), bracket, separator)
		}
		other => as_key(other),
	})
}

/// `name = value` of a plain name as `name: value`; any other item as it is
fn as_key(item: Node) -> Node {
	item.under_meta(|item| match item {
		Node::Key(name, Op::Assign, value) if name.drop_meta().symbol_name().is_some() => {
			Node::Key(name, Op::Colon, value)
		}
		other => other,
	})
}

fn is_entry(item: &Node) -> bool {
	matches!(item.drop_meta(), Node::Key(name, Op::Assign, _) if name.drop_meta().symbol_name().is_some())
}

/// The functions the program defines: `f(x) := …`, `f := …`, `def f {…}`, but no variable `x = 1`
fn defined_functions(program: &Node) -> HashSet<String> {
	let mut names = HashSet::new();
	program.visit(&mut |node| {
		if let Some(head) = function_head(node) {
			let is_variable = head.form == DefinitionForm::Operator(Op::Assign) && head.written_parameters.is_empty();
			if !is_variable {
				names.insert(head.name.to_string());
			}
		}
	});
	names
}
