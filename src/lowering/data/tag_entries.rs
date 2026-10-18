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
use crate::web::markup::names_attribute;
use crate::words::{IMPLICIT_PARAMETER, ON_WORD};
use std::collections::HashSet;

pub fn lower(program: Node) -> Node {
	let functions = defined_functions(&program);
	with_entries(program, &functions)
}

fn with_entries(node: Node, functions: &HashSet<String>) -> Node {
	match node {
		Node::Meta { .. } => node.under_meta(|inner| with_entries(inner, functions)),
		Node::List(items, bracket, separator) => {
			// `label(for="pwd")`, `p {class="note"}`: an element's parentheses, or its block spaced from it
			let is_element_group = items.first().is_some_and(is_element_name) && items.len() > 1;
			let mut after_on = false;
			let items = items.into_iter().enumerate().map(|(index, item)| {
				// `on change { saved = event.value }`: a handler's block is code
				let is_handler = std::mem::replace(&mut after_on, item.is_symbol(ON_WORD));
				let item = match is_handler {
					true => item.under_meta(|handler| handler.map_children(|child| with_entries(child, functions))),
					false => with_entries(item, functions),
				};
				match index > 0 && is_element_group {
					true => element_part(item),
					false => item,
				}
			});
			Node::List(items.collect(), bracket, separator)
		}
		Node::Key(tag, Op::Colon, body) if is_data_tag(&tag, &body, functions) => {
			Node::Key(tag, Op::Colon, Box::new(entries_of(with_entries(*body, functions))))
		}
		other => other.map_children(|child| with_entries(child, functions)),
	}
}

/// A part after an element's name: its `(…)` or `{…}` with entries, an attribute `for = "pwd"` as `for: "pwd"`; a
/// `b = …` naming no attribute stays as written (`a, b = [1, 2]` destructures)
fn element_part(part: Node) -> Node {
	match part.drop_meta() {
		Node::Key(name, Op::Assign, _) if !name.drop_meta().symbol_name().is_some_and(names_attribute) => part,
		_ => entries_of(part),
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
		&& !is_lambda_body(body)
}

/// `{ parent = $1 }`, `{ total = it * 2 }`: a block naming its parameters `it` or `$0`, `$1` … is a lambda
fn is_lambda_body(body: &Node) -> bool {
	let is_parameter = |name: &str| {
		name == IMPLICIT_PARAMETER
			|| name.strip_prefix('$').is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
	};
	let mut named = false;
	body.visit(&mut |part| named |= matches!(part, Node::Symbol(name) if is_parameter(name)));
	named
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
