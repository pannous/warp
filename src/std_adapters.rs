//! The standard library's adapters (notes/stdlib.md section 7, adapter A): the words of std/<module>.wasp that wasp
//! cannot write itself call the host words std_pure / std_io (module, member, arguments), answered here natively and
//! by host.js's twin in the browser. Nodes in, a Node out; a failure is the error naming module.member. JSON crosses as
//! for the foreign runtimes (foreign.rs json_of / node_of, host.js plainOfTree / treeOfPlain): null ø, booleans 1/0.

use crate::node::Node;

/// module.member applied to the arguments (a list node)
pub fn call(module: &str, member: &str, arguments: &Node) -> Result<Node, String> {
	let arguments = arguments_of(arguments);
	let failure = |problem: String| format!("{module}.{member}: {problem}");
	let text_of = |node: &Node| match node.drop_meta() {
		Node::Text(text) => Ok(text.clone()),
		other => Err(failure(format!("needs a text, got {}", other.serialize().trim()))),
	};
	match (module, member, arguments.as_slice()) {
		("json", "parse", [text]) => {
			let text = text_of(text)?;
			let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|problem| failure(format!("{text:?} is no json: {problem}")))?;
			Ok(crate::foreign::node_of(&parsed))
		}
		("json", "to_json", [value]) => Ok(Node::Text(crate::foreign::json_of(value).to_string())),
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
