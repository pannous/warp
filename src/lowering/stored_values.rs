//! Persisted signals (card web-stores, notes/web_framework.md step 12): `stored theme = "dark"` is the variable theme
//! holding the value an earlier run kept under its name, else "dark"; each change of it is kept again. Natively the
//! values live in `<program>.stored.json` next to the program (in memory for code without a file, std_adapters.rs), in
//! the playground in the page's localStorage (host.js). `stored x = v` becomes
//! `x = std_io("store", "load", ["x", v, file])` and `on change x { std_io("store", "save", ["x", value, file]) }`.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::wasp_parser::parse;
use std::collections::HashMap;

pub const STORED_WORD: &str = "stored";
/// `app.wasp` keeps its stored values in `app.stored.json`
const STORE_FILE_EXTENSION: &str = "stored.json";
/// stands for the default value in the template of the load
const DEFAULT_PLACEHOLDER: &str = "stored_default_value";

pub fn lower(program: Node) -> Node {
	if !crate::wasp_parser::mentions(&program, STORED_WORD) || crate::soft_keywords::program_names(&program, STORED_WORD) {
		return program;
	}
	let file = store_file();
	if let Some(lowered) = stored_statement(&program, &file) {
		return Node::List(lowered, Bracket::None, Separator::Semicolon);
	}
	match program {
		Node::Meta { node, data } => Node::Meta { node: Box::new(lower(*node)), data },
		Node::List(statements, bracket @ Bracket::None, separator @ (Separator::Semicolon | Separator::Newline)) => {
			let statements = statements.into_iter().flat_map(|statement| stored_statement(&statement, &file).unwrap_or_else(|| vec![statement])).collect();
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
	let load = parse(&format!("{name} = std_io(\"store\", \"load\", [\"{name}\", {DEFAULT_PLACEHOLDER}, {file:?}])"));
	let load = crate::law::substitute(&load, &HashMap::from([(DEFAULT_PLACEHOLDER.to_string(), default.as_ref().clone())]));
	let save = parse(&format!("on change {name} {{ std_io(\"store\", \"save\", [\"{name}\", value, {file:?}]) }}"));
	Some(vec![load, save])
}
