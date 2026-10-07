//! The standard library's adapters (notes/stdlib.md section 7, adapter A): the words of std/<module>.wasp that wasp
//! cannot write itself call the host words std_pure / std_io (module, member, arguments), answered here natively and
//! by host.js's twin in the browser. Nodes in, a Node out; a failure is the error naming module.member. JSON crosses as
//! for the foreign runtimes (foreign.rs json_of / node_of, host.js plainOfTree / treeOfPlain): null ø, booleans 1/0.

use crate::node::Node;

/// module.member applied to the arguments (a list node)
pub fn call(module: &str, member: &str, arguments: &Node) -> Result<Node, String> {
	let arguments = arguments_of(arguments);
	let failure = |problem: String| format!("{module}.{member}: {problem}");
	// a one-character text arrives as a Char
	let text_of = |node: &Node| match node.drop_meta() {
		Node::Text(text) => Ok(text.clone()),
		Node::Char(character) => Ok(character.to_string()),
		other => Err(failure(format!("needs a text, got {}", other.serialize().trim()))),
	};
	// what write puts into a file: a text as it is, any other value as wasp writes it (`42`, `[1 2]`)
	let content_of = |node: &Node| text_of(node).or_else(|_| Ok::<String, String>(node.serialize().trim().to_string()));
	match (module, member, arguments.as_slice()) {
		("json", "parse", [text]) => {
			let text = text_of(text)?;
			let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|problem| failure(format!("{text:?} is no json: {problem}")))?;
			Ok(crate::foreign::node_of(&parsed))
		}
		("json", "to_json", [value]) => Ok(Node::Text(crate::foreign::json_of(value).to_string())),
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
		("net", "post", [url, body]) => crate::extensions::utils::post_within(&text_of(url)?, &content_of(body)?, crate::host::FETCH_TIMEOUT).map(Node::Text).map_err(failure),
		("os", "env", [name]) => Ok(std::env::var(text_of(name)?).map_or(Node::Empty, Node::Text)),
		_ => Err(failure(format!("no such word of {} arguments", arguments.len()))),
	}
}

/// A list of texts
fn texts(items: impl IntoIterator<Item = String>) -> Node {
	Node::List(items.into_iter().map(Node::Text).collect(), crate::node::Bracket::Square, crate::node::Separator::Space)
}

/// The pattern, if both engines read it alike: Rust's regex has no look-around or backreferences, so JS RegExp may not
/// use them either (host.js checks the same); every other syntax error is the regex crate's own
fn regex_of(pattern: &str) -> Result<regex::Regex, String> {
	if let Some(feature) = unshared_feature(pattern) {
		return Err(format!("{feature} is not in wasp's regex (one engine lacks it): {pattern}"));
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

fn arguments_of(arguments: &Node) -> Vec<Node> {
	match arguments.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	}
}
