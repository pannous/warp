//! Persisted signals (card web-stores, notes/web_framework.md step 12): `stored theme = "dark"` is the variable theme
//! holding the value an earlier run kept under its name, else "dark"; each change of it is kept again. Natively the
//! values live in `<program>.stored.json` next to the program (in memory for code without a file, std_adapters.rs), in
//! the playground in the page's localStorage (host.js). `stored x = v` becomes
//! `x = std_io("store", "load", ["x", v, file])` and `on change x { std_io("store", "save", ["x", value, file]) }`.
//! `storage` is the same store as a map keyed at run time (card web-apis): `storage[k] = v` saves, `storage[k]` loads (ø
//! when absent), `delete storage[k]` removes, `keys(storage)` names the kept values; `storage.k` is the key "k".
//! A program that defines its own `storage` keeps it.

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
pub const STORAGE_WORD: &str = "storage";
const DELETE_WORD: &str = "delete";
const KEYS_WORD: &str = "keys";
/// stand for the key and the value in the templates of a storage access
const KEY_PLACEHOLDER: &str = "storage_key";
const VALUE_PLACEHOLDER: &str = "storage_value";

pub fn lower(program: Node) -> Node {
	let program = if uses_storage(&program) { storage_accesses(program, &store_file()) } else { program };
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

fn uses_storage(program: &Node) -> bool {
	crate::wasp_parser::mentions(program, STORAGE_WORD) && !crate::soft_keywords::program_names(program, STORAGE_WORD)
}

/// Each access of `storage` as the store's call
fn storage_accesses(node: Node, file: &str) -> Node {
	storage_access(&node, file).unwrap_or_else(|| node.map_children(|child| storage_accesses(child, file)))
}

fn storage_access(node: &Node, file: &str) -> Option<Node> {
	match node.drop_meta() {
		Node::Key(target, Op::Assign, value) => {
			let key = entry_key(target)?;
			Some(store_call("save", Some(key), Some(storage_accesses(value.as_ref().clone(), file)), file))
		}
		Node::List(items, _, _) => match items.as_slice() {
			[word, target] if is_word(word, DELETE_WORD) => Some(store_call("remove", Some(entry_key(target)?), None, file)),
			[word, store] if is_word(word, KEYS_WORD) && is_word(store, STORAGE_WORD) => Some(store_call("names", None, None, file)),
			_ => None,
		},
		// ø: the value of an absent key
		_ => Some(store_call("load", Some(entry_key(node)?), Some(Node::Empty), file)),
	}
}

/// The key of `storage[k]` (parsed as `storage#(k+1)`), `storage#k` or `storage.k`
fn entry_key(target: &Node) -> Option<Node> {
	let Node::Key(store, op, index) = target.drop_meta() else { return None };
	if !is_word(store, STORAGE_WORD) {
		return None;
	}
	match (op, index.drop_meta()) {
		(Op::Dot, Node::Symbol(field)) => Some(Node::Text(field.clone())),
		(Op::Hash, Node::Key(key, Op::Add, one)) if *one.drop_meta() == Node::int(1) => Some(key.as_ref().clone()),
		(Op::Hash, key) => Some(key.clone()),
		_ => None,
	}
}

fn is_word(node: &Node, word: &str) -> bool {
	matches!(node.drop_meta(), Node::Symbol(name) if name == word)
}

/// `std_io("store", member, [key, value, file])`, the key and the value given
fn store_call(member: &str, key: Option<Node>, value: Option<Node>, file: &str) -> Node {
	let given = [(KEY_PLACEHOLDER, key), (VALUE_PLACEHOLDER, value)];
	let arguments: Vec<String> = given.iter().filter(|(_, node)| node.is_some()).map(|(name, _)| name.to_string()).chain([format!("{file:?}")]).collect();
	let call = parse(&format!("std_io(\"store\", \"{member}\", [{}])", arguments.join(", ")));
	let values = given.into_iter().filter_map(|(name, node)| Some((name.to_string(), node?))).collect();
	crate::law::substitute(&call, &values)
}
