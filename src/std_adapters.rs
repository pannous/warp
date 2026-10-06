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
			Ok(Node::List(names.into_iter().map(Node::Text).collect(), crate::node::Bracket::Square, crate::node::Separator::Space))
		}
		("os", "env", [name]) => Ok(std::env::var(text_of(name)?).map_or(Node::Empty, Node::Text)),
		_ => Err(failure(format!("no such word of {} arguments", arguments.len()))),
	}
}

fn arguments_of(arguments: &Node) -> Vec<Node> {
	match arguments.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	}
}
