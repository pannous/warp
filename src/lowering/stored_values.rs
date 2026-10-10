//! Persisted signals (card web-stores, notes/web_framework.md step 12): `stored theme = "dark"` is the variable theme
//! holding the value an earlier run kept under its name, else "dark"; each change of it is kept again. Natively the
//! values live in `<program>.stored.json` next to the program (in memory for code without a file, std_adapters.rs), in
//! the playground in the page's localStorage (host.js). `stored x = v` becomes
//! `x = std_io("store", "load", ["x", v, file])` and `on change x { std_io("store", "save", ["x", value, file]) }`.
//! `storage` is the same store as a map keyed at run time (card web-apis): `storage[k] = v` saves, `storage[k]` loads (ø
//! when absent), `delete storage[k]` removes, `keys(storage)` names the kept values; `storage.k` is the key "k".
//! `local` is that store under its browser name (localStorage; `storage` its alias), `session` a store of its own
//! (sessionStorage: while the page's tab lasts; natively while the process runs) (P188, warp-03's default).
//! A program that defines its own `storage`, `local` or `session` keeps it.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;
use std::collections::{HashMap, HashSet};

pub const STORED_WORD: &str = "stored";
/// The store of the values a `warp dev` page keeps across reloads (in memory natively; dev.js, site.js)
pub const DEV_STORE: &str = "warp-dev";
/// `app.warp` keeps its stored values in `app.stored.json`
const STORE_FILE_EXTENSION: &str = "stored.json";
/// stands for the default value in the template of the load
const DEFAULT_PLACEHOLDER: &str = "stored_default_value";
/// The words of the stores keyed at run time: the program's store (localStorage) and the session's
const LOCAL_WORDS: [&str; 2] = ["local", "storage"];
const SESSION_WORD: &str = "session";
/// The store of `session[k]` (in memory natively, std_adapters.rs; sessionStorage in the browser, markup.js SESSION_STORE)
pub const SESSION_STORE: &str = "warp-session";
/// `database[k]` (alias indexedDB): the store for values beyond localStorage's ~5 MB. Its file ends in this, `app.warp`'s
/// is `app.database.json`, inline code's this name alone (in memory natively, std_adapters.rs); in the browser any
/// store whose file ends so is IndexedDB (host-files.js DATABASE_STORE)
pub const DATABASE_STORE: &str = "database.json";
const DATABASE_WORDS: [&str; 2] = ["database", "indexedDB"];
const DELETE_WORD: &str = "delete";
const KEYS_WORD: &str = "keys";
/// stand for the key and the value in the templates of a storage access
const KEY_PLACEHOLDER: &str = "storage_key";
const VALUE_PLACEHOLDER: &str = "storage_value";

pub fn lower(program: Node) -> Node {
	let stores = stores_used(&program);
	let program = if stores.is_empty() { program } else { storage_accesses(program, &stores) };
	let stores = crate::warp_parser::mentions(&program, STORED_WORD) && !crate::soft_keywords::program_names(&program, STORED_WORD);
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
	program_file_with(STORE_FILE_EXTENSION).unwrap_or_default()
}

/// The program's database file, DATABASE_STORE for inline code
fn database_file() -> String {
	program_file_with(DATABASE_STORE).unwrap_or_else(|| DATABASE_STORE.to_string())
}

fn program_file_with(extension: &str) -> Option<String> {
	crate::modules::program_file().map(|file| file.with_extension(extension).to_string_lossy().into_owned())
}

/// `stored x = v` as its load and the listener that saves each change
fn stored_statement(statement: &Node, file: &str) -> Option<Vec<Node>> {
	let Node::List(items, _, _) = statement.drop_meta() else { return None };
	let [word, assignment] = items.as_slice() else { return None };
	if !word.is_symbol(STORED_WORD) {
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

/// The store words the program uses and does not name itself, with the file of each store
fn stores_used(program: &Node) -> Vec<(&'static str, String)> {
	let (local, database) = (store_file(), database_file());
	let words = LOCAL_WORDS.iter().map(|word| (*word, local.clone())).chain([(SESSION_WORD, SESSION_STORE.to_string())]);
	let words = words.chain(DATABASE_WORDS.iter().map(|word| (*word, database.clone())));
	words.filter(|(word, _)| crate::warp_parser::mentions(program, word) && !crate::soft_keywords::program_names(program, word)).collect()
}

/// Each access of a store word as the store's call
fn storage_accesses(node: Node, stores: &[(&str, String)]) -> Node {
	storage_access(&node, stores).unwrap_or_else(|| node.map_children(|child| storage_accesses(child, stores)))
}

fn storage_access(node: &Node, stores: &[(&str, String)]) -> Option<Node> {
	match node.drop_meta() {
		Node::Key(target, Op::Assign, value) => {
			let (key, file) = entry_key(target, stores)?;
			Some(store_call("save", Some(key), Some(storage_accesses(value.as_ref().clone(), stores)), file))
		}
		Node::List(items, _, _) => match items.as_slice() {
			[word, target] if word.is_symbol(DELETE_WORD) => {
				let (key, file) = entry_key(target, stores)?;
				Some(store_call("remove", Some(key), None, file))
			}
			[word, store] if word.is_symbol(KEYS_WORD) => Some(store_call("names", None, None, file_of(store, stores)?)),
			_ => None,
		},
		// ø: the value of an absent key
		_ => {
			let (key, file) = entry_key(node, stores)?;
			Some(store_call("load", Some(key), Some(Node::Empty), file))
		}
	}
}

/// The key and the store's file of `local[k]` (parsed as `local#(k+1)`), `local#k` or `local.k`
fn entry_key<'a>(target: &Node, stores: &'a [(&str, String)]) -> Option<(Node, &'a str)> {
	let Node::Key(store, op, index) = target.drop_meta() else { return None };
	let file = file_of(store, stores)?;
	let key = match (op, index.drop_meta()) {
		(Op::Dot, Node::Symbol(field)) => Node::Text(field.clone()),
		(Op::Hash, Node::Key(key, Op::Add, one)) if *one.drop_meta() == Node::int(1) => key.as_ref().clone(),
		(Op::Hash, key) => key.clone(),
		_ => return None,
	};
	Some((key, file))
}

/// The file of the store a word names
fn file_of<'a>(word: &Node, stores: &'a [(&str, String)]) -> Option<&'a str> {
	stores.iter().find(|(name, _)| word.is_symbol(name)).map(|(_, file)| file.as_str())
}

/// `std_io("store", member, [key, value, file])`, the key and the value given
fn store_call(member: &str, key: Option<Node>, value: Option<Node>, file: &str) -> Node {
	let given = [(KEY_PLACEHOLDER, key), (VALUE_PLACEHOLDER, value)];
	let arguments: Vec<String> = given.iter().filter(|(_, node)| node.is_some()).map(|(name, _)| name.to_string()).chain([format!("{file:?}")]).collect();
	let call = parse(&format!("std_io(\"store\", \"{member}\", [{}])", arguments.join(", ")));
	let values = given.into_iter().filter_map(|(name, node)| Some((name.to_string(), node?))).collect();
	crate::law::substitute(&call, &values)
}
