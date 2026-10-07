//! A page renders itself (card web-ssr, notes/web_framework.md "Built sites"): a program compiled for a page
//! (pipeline::for_a_page) exports page·html, the HTML of what it shows by lib/markup.wasp's to_html, which `warp build
//! --site` calls at build time for index.html and the page after each handler. What it shows is page·value, the output
//! binding event_signals.rs made, else the program's last line when that is an expression: a page shows its last line
//! anew, so it is read again after each handler.

use crate::event_signals::{function_with_globals, main_level_variables, PAGE_VALUE};
use crate::node::Node;
use crate::system_signals::call;
use crate::operators::Op;
use std::collections::HashSet;

pub const PAGE_HTML: &str = "page·html";
/// page·render(value): the HTML of any value of the program, for the playground's fine holes (worker.js showHoles)
pub const PAGE_RENDER: &str = "page·render";
const MARKUP_MODULE_USE: &str = "use markup";
const TO_HTML: &str = "to_html";
/// Words that start a statement, not a value to show
const STATEMENT_WORDS: [&str; 5] = ["print", "puts", "use", "import", "return"];

/// A page uses lib/markup.wasp and exports page·html := to_html(page·value()) and page·render(event) := to_html(event),
/// before modules::resolve joins the used modules (it keeps the std definitions the program names)
pub fn use_markup(program: Node) -> Node {
	if !crate::pipeline::renders_itself() {
		return program;
	}
	let (mut statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	let rendered = call(TO_HTML, vec![call(PAGE_VALUE, vec![])]);
	statements.insert(statements.len().saturating_sub(1), function_with_globals(PAGE_HTML, false, &[rendered], &HashSet::new()));
	let any_value = call(TO_HTML, vec![Node::Symbol(crate::event_signals::EVENT_WORD.to_string())]);
	statements.insert(statements.len().saturating_sub(1), function_with_globals(PAGE_RENDER, true, &[any_value], &HashSet::new()));
	statements.insert(0, crate::wasp_parser::parse(MARKUP_MODULE_USE));
	Node::List(statements, bracket, separator)
}

/// page·value is the output binding event_signals made, else the last line when that is a value; without one the page
/// shows nothing and page·html goes again
pub fn lower(program: Node) -> Node {
	if !crate::pipeline::renders_itself() {
		return program;
	}
	let (mut statements, bracket, separator) = crate::variable_signals::main_statements(&program);
	if statements.iter().any(|statement| defines(statement, PAGE_VALUE)) {
		return program;
	}
	match statements.last().filter(|last| is_shown(last)).cloned() {
		Some(shown) => {
			let main_variables = main_level_variables(&statements);
			statements.insert(statements.len() - 1, function_with_globals(PAGE_VALUE, false, &[shown], &main_variables));
		}
		None => statements.retain(|statement| !defines(statement, PAGE_HTML)),
	}
	Node::List(statements, bracket, separator)
}

/// `name() := …`, `(name) := …`
fn defines(statement: &Node, name: &str) -> bool {
	let Node::Key(head, Op::Define, _) = statement.drop_meta() else { return false };
	let head = match head.drop_meta() {
		Node::List(items, _, _) => items.first().map(Node::drop_meta),
		other => Some(other),
	};
	matches!(head, Some(Node::Symbol(defined)) if defined == name)
}

/// A last line that is a value: no assignment, definition or statement word
fn is_shown(statement: &Node) -> bool {
	match statement.drop_meta() {
		Node::Key(_, op, _) => !matches!(op, Op::Assign | Op::Define) && !op.is_compound_assign(),
		Node::List(items, _, _) => !matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(word)) if STATEMENT_WORDS.contains(&word.as_str())),
		_ => true,
	}
}
