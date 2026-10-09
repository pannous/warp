//! The standard library's adapters (notes/stdlib.md section 7, adapter A): the words of lib/<module>.warp that warp
//! cannot write itself call the host words std_pure / std_io (module, member, arguments), answered here natively and
//! by host.js's twin in the browser. Nodes in, a Node out; a failure is the error naming module.member. JSON crosses as
//! for the foreign runtimes (foreign.rs json_of / node_of, host.js plainOfTree / treeOfPlain): null ø, booleans 1/0.

use crate::node::Node;

/// The text a host word's argument holds: a one-character text arrives as a Char (`"n"` parses as 'n')
pub(crate) fn text_value(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Text(text) => Some(text.clone()),
		Node::Char(character) => Some(character.to_string()),
		_ => None,
	}
}

/// module.member applied to the arguments (a list node)
pub fn call(module: &str, member: &str, arguments: &Node) -> Result<Node, String> {
	let arguments = arguments_of(arguments);
	let failure = |problem: String| format!("{module}.{member}: {problem}");
	let text_of = |node: &Node| text_value(node).ok_or_else(|| failure(format!("needs a text, got {}", node.serialize().trim())));
	// what write puts into a file: a text as it is, any other value as warp writes it (`42`, `[1 2]`)
	let content_of = |node: &Node| text_of(node).or_else(|_| Ok::<String, String>(node.serialize().trim().to_string()));
	match (module, member, arguments.as_slice()) {
		("json", "parse", [text]) => {
			let text = text_of(text)?;
			let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|problem| failure(format!("{text:?} is no json: {problem}")))?;
			Ok(crate::foreign::node_of(&parsed))
		}
		("json", "to_json", [value]) => Ok(Node::Text(crate::foreign::json_of(value).to_string())),
		// an instance of one of the program's classes is its fields: `{"Point": {"x": 1}}` is `{"x": 1}` (as host.js)
		("json", "to_json", [value, classes]) => {
			let classes: Vec<String> = classes.children().iter().filter_map(|class| text_of(class).ok()).collect();
			Ok(Node::Text(without_class_tags(crate::foreign::json_of(value), &classes).to_string()))
		}
		("file", "write", [path, text]) => std::fs::write(text_of(path)?, content_of(text)?).map(|_| Node::Empty).map_err(|problem| failure(problem.to_string())),
		("file", "append", [path, text]) => {
			use std::io::Write;
			let mut file = std::fs::OpenOptions::new().append(true).create(true).open(text_of(path)?).map_err(|problem| failure(problem.to_string()))?;
			file.write_all(content_of(text)?.as_bytes()).map(|_| Node::Empty).map_err(|problem| failure(problem.to_string()))
		}
		// a truth value crosses as 1 or 0, as booleans are encoded
		("file", "exists", [path]) => Ok(Node::int(i64::from(std::path::Path::new(&text_of(path)?).exists()))),
		("file", "list", [folder]) => {
			let entries = std::fs::read_dir(text_of(folder)?).map_err(|problem| failure(problem.to_string()))?;
			let mut names: Vec<String> = entries.filter_map(|entry| Some(entry.ok()?.file_name().to_string_lossy().into_owned())).collect();
			names.sort();
			Ok(texts(names))
		}
		("hash", "sha256", [subject]) => {
			use sha2::Digest;
			Ok(Node::Text(sha2::Sha256::digest(content_of(subject)?.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()))
		}
		("hash", "crc32", [subject]) => Ok(Node::int(i64::from(crc32fast::hash(content_of(subject)?.as_bytes())))),
		("regex", member, [text, pattern, rest @ ..]) => {
			let (text, pattern) = (text_of(text)?, regex_of(&text_of(pattern)?).map_err(failure)?);
			match (member, rest) {
				("matches", []) => Ok(Node::int(i64::from(pattern.is_match(&text)))),
				("first", []) => Ok(pattern.find(&text).map_or(Node::Empty, |found| Node::Text(found.as_str().to_string()))),
				("all", []) => Ok(texts(pattern.find_iter(&text).map(|found| found.as_str().to_string()))),
				("replace", [replacement]) => Ok(Node::Text(pattern.replace_all(&text, text_of(replacement)?.as_str()).into_owned())),
				_ => Err(failure(format!("no such word of {} arguments", arguments.len()))),
			}
		}
		("net", "post", [url, body]) => crate::extensions::utils::post_within(&text_of(url)?, &content_of(body)?, &[], crate::host::FETCH_TIMEOUT).map(Node::Text).map_err(failure),
		// `post(url, body, {"x-api-key": key})`: the map's pairs are the request's headers
		("net", "post", [url, body, headers]) => {
			let headers = header_pairs(headers).map_err(failure)?;
			crate::extensions::utils::post_within(&text_of(url)?, &content_of(body)?, &headers, crate::host::FETCH_TIMEOUT).map(Node::Text).map_err(failure)
		}
		// `clipboard.write(text)` (lowering/system_values.rs)
		#[cfg(feature = "native")]
		("clipboard", "write", [text]) => warp_runtime::system_values::write_clipboard(&text_of(text)?).map(|_| Node::Empty).map_err(failure),
		("os", "env", [name]) => Ok(std::env::var(text_of(name)?).map_or(Node::Empty, Node::Text)),
		("os", "args", []) => Ok(texts(PROGRAM_ARGUMENTS.with(|arguments| arguments.borrow().clone()))),
		// `stored theme = "dark"` (lowering/stored_values.rs): the value kept under its name, or the default
		("store", "load", [name, default, file]) => {
			let kept = stored_values(&text_of(file)?).map_err(failure)?.remove(&text_of(name)?);
			Ok(kept.map_or_else(|| default.clone(), |value| crate::foreign::node_of(&value)))
		}
		("store", "save", [name, value, file]) => {
			let file = text_of(file)?;
			let mut values = stored_values(&file).map_err(failure)?;
			values.insert(text_of(name)?, crate::foreign::json_of(value));
			save_stored_values(&file, values).map(|_| Node::Empty).map_err(failure)
		}
		// `delete storage[k]`, `keys(storage)` (lowering/stored_values.rs)
		("store", "remove", [name, file]) => {
			let file = text_of(file)?;
			let mut values = stored_values(&file).map_err(failure)?;
			values.remove(&text_of(name)?);
			save_stored_values(&file, values).map(|_| Node::Empty).map_err(failure)
		}
		// the tables of registered classes (lowering/database_tables.rs)
		#[cfg(feature = "native")]
		("table", member, arguments) => crate::database::call(member, arguments).map_err(failure),
		("store", "names", [file]) => Ok(texts(stored_values(&text_of(file)?).map_err(failure)?.into_iter().map(|(name, _)| name))),
		_ => Err(failure(format!("no such word of {} arguments", arguments.len()))),
	}
}

type StoredValues = serde_json::Map<String, serde_json::Value>;

thread_local! {
	/// The command line arguments after the program file (`warp run prog.warp a b`), what `use os; args` gives
	static PROGRAM_ARGUMENTS: std::cell::RefCell<Vec<String>> = Default::default();
}

/// The arguments the running program gets as `args`
pub fn set_program_arguments(arguments: Vec<String>) {
	PROGRAM_ARGUMENTS.with(|kept| *kept.borrow_mut() = arguments);
}

thread_local! {
	/// The stored values kept in memory while the process runs, by store: of a program without a file (`warp eval`,
	/// tests) and a dev page's under "", the session's (`session[k]`) under its own
	static UNFILED_STORES: std::cell::RefCell<std::collections::HashMap<&'static str, StoredValues>> = Default::default();
}

/// The store in memory a file stands for: of a program without a file, those a `warp dev` page keeps itself (its
/// sessionStorage), the session's, inline code's database; none for a store file
fn unfiled_store(file: &str) -> Option<&'static str> {
	match file {
		"" | crate::stored_values::DEV_STORE => Some(""),
		crate::stored_values::SESSION_STORE => Some(crate::stored_values::SESSION_STORE),
		crate::stored_values::DATABASE_STORE => Some(crate::stored_values::DATABASE_STORE),
		_ => None,
	}
}

/// The stored values in the program's store file (a JSON object by name), none when it does not exist yet
fn stored_values(file: &str) -> Result<StoredValues, String> {
	if let Some(store) = unfiled_store(file) {
		return Ok(UNFILED_STORES.with(|stores| stores.borrow().get(store).cloned().unwrap_or_default()));
	}
	match std::fs::read_to_string(file) {
		Ok(text) => serde_json::from_str(&text).map_err(|problem| format!("{file} holds no stored values: {problem}")),
		Err(problem) if problem.kind() == std::io::ErrorKind::NotFound => Ok(StoredValues::new()),
		Err(problem) => Err(format!("{file}: {problem}")),
	}
}

fn save_stored_values(file: &str, values: StoredValues) -> Result<(), String> {
	if let Some(store) = unfiled_store(file) {
		UNFILED_STORES.with(|stores| stores.borrow_mut().insert(store, values));
		return Ok(());
	}
	let text = serde_json::to_string_pretty(&values).map_err(|problem| problem.to_string())?;
	std::fs::write(file, text).map_err(|problem| format!("{file}: {problem}"))
}

/// A list of texts
fn texts(items: impl IntoIterator<Item = String>) -> Node {
	Node::List(items.into_iter().map(Node::Text).collect(), crate::node::Bracket::Square, crate::node::Separator::Space)
}

/// The pattern, if both engines read it alike: Rust's regex has no look-around or backreferences, so JS RegExp may not
/// use them either (host.js checks the same); every other syntax error is the regex crate's own
fn regex_of(pattern: &str) -> Result<regex::Regex, String> {
	if let Some(feature) = unshared_feature(pattern) {
		return Err(format!("{feature} is not in warp's regex (one engine lacks it): {pattern}"));
	}
	regex::Regex::new(pattern).map_err(|problem| problem.to_string())
}

/// look-around `(?=`, `(?!`, `(?<=`, `(?<!` or a backreference `\1`
fn unshared_feature(pattern: &str) -> Option<&'static str> {
	let look_around = ["(?=", "(?!", "(?<=", "(?<!"].iter().any(|opening| pattern.contains(opening));
	let backreference = pattern.as_bytes().windows(2).any(|pair| pair[0] == b'\\' && pair[1].is_ascii_digit() && pair[1] != b'0');
	match (look_around, backreference) {
		(true, _) => Some("look-around"),
		(_, true) => Some("a backreference"),
		_ => None,
	}
}

/// A map of texts `{"x-api-key": key}` as (name, value) pairs
fn header_pairs(headers: &Node) -> Result<Vec<(String, String)>, String> {
	let serde_json::Value::Object(pairs) = crate::foreign::json_of(headers) else {
		return Err(format!("headers are a map of texts, got {}", headers.serialize().trim()));
	};
	pairs.into_iter().map(|(name, value)| match value {
		serde_json::Value::String(text) => Ok((name, text)),
		other => Err(format!("header {name} needs a text, got {other}")),
	}).collect()
}

fn arguments_of(arguments: &Node) -> Vec<Node> {
	match arguments.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	}
}

/// The json with each object `{"Point": {…}}` of a class named in `classes` as its fields `{…}`
fn without_class_tags(value: serde_json::Value, classes: &[String]) -> serde_json::Value {
	use serde_json::Value;
	match value {
		Value::Array(items) => Value::Array(items.into_iter().map(|item| without_class_tags(item, classes)).collect()),
		Value::Object(entries) => match entries.len() == 1 && entries.iter().next().is_some_and(|(key, fields)| classes.contains(key) && fields.is_object()) {
			true => without_class_tags(entries.into_iter().next().map(|(_, fields)| fields).unwrap_or_default(), classes),
			false => Value::Object(entries.into_iter().map(|(key, entry)| (key, without_class_tags(entry, classes))).collect()),
		},
		other => other,
	}
}
