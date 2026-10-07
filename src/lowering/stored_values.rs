//! Persisted signals (card web-stores, notes/web_framework.md step 12): `stored theme = "dark"` is the variable theme
//! holding the value an earlier run kept under its name, else "dark"; each change of it is kept again. Natively the
//! values live in `<program>.stored.json` next to the program (in memory for code without a file, std_adapters.rs), in
//! the playground in the page's localStorage (host.js). `stored x = v` becomes
//! `x = std_io("store", "load", ["x", v, file])` and `on change x { std_io("store", "save", ["x", value, file]) }`.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::parse;
use std::collections::{HashMap, HashSet};

pub const STORED_WORD: &str = "stored";
/// The store of the values a `warp dev` page keeps across reloads (in memory natively; dev.js, site.js)
pub const DEV_STORE: &str = "wasp-dev";
/// `app.wasp` keeps its stored values in `app.stored.json`
const STORE_FILE_EXTENSION: &str = "stored.json";
/// stands for the default value in the template of the load
const DEFAULT_PLACEHOLDER: &str = "stored_default_value";

pub fn lower(program: Node) -> Node {
	let stores = crate::wasp_parser::mentions(&program, STORED_WORD) && !crate::soft_keywords::program_names(&program, STORED_WORD);
	let dev = crate::pipeline::is_for_dev();
	if !stores && !dev {
		return program;
	}
	let file = store_file();
	if let Some(lowered) = stores.then(|| stored_statement(&program, &file)).flatten() {
		return Node::List(lowered, Bracket::None, Separator::Semicolon);
	}
	match program {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		Node::List(statements, bracket @ Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			let changed = if dev { changed_names(&statements) } else { HashSet::new() };
			let mut kept_names = HashSet::new();
			let statements = statements.into_iter().flat_map(|statement| {
				let stored = stores.then(|| stored_statement(&statement, &file)).flatten();
				stored.or_else(|| dev_kept(&statement, &changed, &mut kept_names)).unwrap_or_else(|| vec![statement])
			}).collect();
			Node::List(statements, bracket, separator)
		}
		other => other,
	}
}

/// The program's store file, "" for inline code
fn store_file() -> String {
	crate::modules::program_file().map(|file| file.with_extension(STORE_FILE_EXTENSION).to_string_lossy().into_owned()).unwrap_or_default()
}

/// `stored x = v` as its load and the listener that saves each change
fn stored_statement(statement: &Node, file: &str) -> Option<Vec<Node>> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [word, assignment] = items.as_slice() else { return None };
	if !matches!(word.drop_meta(), Node::Symbol(word) if word == STORED_WORD) {
		return None;
	}
	let Node::Key(name, Op::Assign | Op::Define, default) = assignment.drop_meta() else { return None };
	let Node::Symbol(name) = name.drop_meta() else { return None };
	Some(kept(name, default, file))
}

/// In a `warp dev` build, the first main-level `x = v` of a variable the program changes: kept as `stored` keeps it, in
/// the dev store (the page's sessionStorage), so the page shows the same state after a reload
fn dev_kept(statement: &Node, changed: &HashSet<String>, kept_names: &mut HashSet<String>) -> Option<Vec<Node>> {
	let (name, default) = main_level_assignment(statement)?;
	(changed.contains(name) && kept_names.insert(name.clone())).then(|| kept(name, default, DEV_STORE))
}

/// `x = std_io("store", "load", ["x", v, file])` and the listener saving each change of x
fn kept(name: &str, default: &Node, file: &str) -> Vec<Node> {
	let load = parse(&format!("{name} = std_io(\"store\", \"load\", [\"{name}\", {DEFAULT_PLACEHOLDER}, {file:?}])"));
	let load = crate::law::substitute(&load, &HashMap::from([(DEFAULT_PLACEHOLDER.to_string(), default.clone())]));
	let save = parse(&format!("on change {name} {{ std_io(\"store\", \"save\", [\"{name}\", value, {file:?}]) }}"));
	vec![load, save]
}

fn main_level_assignment(statement: &Node) -> Option<(&String, &Node)> {
	let Node::Key(target, Op::Assign, value) = statement.drop_meta() else { return None };
	let Node::Symbol(name) = target.drop_meta() else { return None };
	Some((name, value))
}

/// The variables the program changes after their first main-level assignment: assigned again or by `+=` and the like
fn changed_names(statements: &[Node]) -> HashSet<String> {
	let mut assigned = HashSet::new();
	let mut changed = HashSet::new();
	let mut collect = |node: &Node| node.visit(&mut |part| {
		if let Node::Key(target, op, _) = part.drop_meta() {
			if let (Node::Symbol(name), true) = (target.drop_meta(), *op == Op::Assign || op.is_compound_assign()) {
				changed.insert(name.clone());
			}
		}
	});
	for statement in statements {
		match main_level_assignment(statement) {
			Some((name, value)) if assigned.insert(name.clone()) => collect(value),
			_ => collect(statement),
		}
	}
	changed
}
