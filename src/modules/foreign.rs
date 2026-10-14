//! The host side of foreign_call(runtime, module, member, arguments) (lowering/foreign/foreign_modules.rs,
//! notes/stdlib_connectors.md): python and js go to the bridge warp_runtime::foreign shares with executables (its
//! children, JSON lines, handles and callbacks), components to src/modules/components.rs; Nodes cross as JSON (json_of, node_of).

use crate::extensions::numbers::Number;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use serde_json::Value;
pub use warp_runtime::foreign::{BIG_INTEGER_KEY, CODEPOINT_KEY, WARP_FUNCTION_KEY};

/// How a call's warp functions are called back: by their index, with the arguments the runtime gave them as a list
pub type Callback<'a> = &'a mut dyn FnMut(usize, Node) -> Result<Node, String>;

/// `runtime.module.member(arguments…)`, or the member itself when it is no call, as a Node; `module` is a module's name
/// or a handle
pub fn call(
	runtime: &str,
	module: &Node,
	member: &str,
	is_call: bool,
	arguments: &Node,
	callback: Callback,
) -> Result<Node, String> {
	let arguments = match arguments.drop_meta() {
		_ if !is_call => Value::Null,
		Node::Empty => Value::Array(vec![]),
		Node::List(items, _, _) => Value::Array(items.iter().map(json_of).collect()),
		single => Value::Array(vec![json_of(single)]),
	};
	if runtime == crate::foreign_modules::COMPONENT_RUNTIME {
		return component_call(module, member, &arguments);
	}
	if !warp_runtime::foreign::is_interpreted(runtime) {
		return Err(format!(
			"no foreign runtime {runtime}: known are {}",
			crate::foreign_modules::FOREIGN_RUNTIMES.join(", ")
		));
	}
	let (module, module_json) = match module.drop_meta() {
		Node::Text(name) | Node::Symbol(name) => (name.clone(), Value::String(name.clone())),
		handle => (handle.serialize(), json_of(handle)),
	};
	let mut callback_on_values =
		|index: usize, arguments: Value| callback(index, node_of(&arguments)).map(|node| json_of(&node));
	warp_runtime::foreign::call(runtime, &module, module_json, member, arguments, &mut callback_on_values)
		.map(|value| node_of(&value))
}

/// `lib.f(x)` of a component (src/modules/components.rs), or `handle.method(x)` of a resource it gave: only calls
fn component_call(module: &Node, member: &str, arguments: &Value) -> Result<Node, String> {
	let receiver = match module.drop_meta() {
		Node::Text(path) | Node::Symbol(path) => path.clone(),
		handle => {
			json_of(handle).get("text").and_then(Value::as_str).map_or_else(|| handle.serialize(), str::to_string)
		}
	};
	let Value::Array(arguments) = arguments else {
		return Err(format!("wasm {receiver}: {member} is a function of the component, call it: {member}(…)"));
	};
	let result = match module.drop_meta() {
		Node::Text(path) | Node::Symbol(path) => crate::modules::components::call(path, member, arguments),
		handle => crate::modules::components::call_method(&json_of(handle), member, arguments),
	};
	result.map(|value| node_of(&value)).map_err(|failure| format!("wasm {receiver}: {failure}"))
}

pub(crate) fn json_of(node: &Node) -> Value {
	match node.drop_meta() {
		Node::List(items, Bracket::Square | Bracket::Round | Bracket::None, _) => {
			Value::Array(items.iter().map(json_of).collect())
		}
		Node::Key(key, Op::Colon, value) => serde_json::json!({ key.name(): json_of(value) }),
		// an object, a handle among them: `{$handle: 7, …}`
		Node::List(items, Bracket::Curly, _)
			if items.iter().all(|item| matches!(item.drop_meta(), Node::Key(_, Op::Colon, _))) =>
		{
			Value::Object(
				items
					.iter()
					.filter_map(|item| match item.drop_meta() {
						Node::Key(key, _, value) => Some((key.name(), json_of(value))),
						_ => None,
					})
					.collect(),
			)
		}
		// an exact rational (`2.5` is 5/2) crosses as a float, an integer as itself
		Node::Number(Number::Int(integer)) => Value::from(*integer),
		Node::Number(number) => Value::from(f64::from(*number)),
		other => serde_json::from_str(&other.to_json_compact().unwrap_or_default()).unwrap_or(Value::Null),
	}
}

/// The character of `{"$char": "b"}`
fn codepoint_of(entries: &serde_json::Map<String, Value>) -> Option<char> {
	let mut characters = entries.get(CODEPOINT_KEY)?.as_str()?.chars();
	match (entries.len(), characters.next(), characters.next()) {
		(1, Some(character), None) => Some(character),
		_ => None,
	}
}

/// A JSON value as a Node: null ø, booleans yes/no, objects `{key:value …}`
pub(crate) fn node_of(value: &Value) -> Node {
	match value {
		Value::Null => Node::Empty,
		Value::Bool(truth) => Node::from(*truth),
		Value::Number(number) => match number.as_i64() {
			Some(integer) => Node::Number(Number::Int(integer)),
			None => Node::Number(Number::Float(number.as_f64().unwrap_or(f64::NAN))),
		},
		Value::String(text) => Node::Text(text.clone()),
		Value::Array(items) => Node::List(items.iter().map(node_of).collect(), Bracket::Square, Separator::Space),
		Value::Object(entries) if entries.len() == 1 && entries.get(BIG_INTEGER_KEY).is_some_and(Value::is_string) => {
			let digits = entries[BIG_INTEGER_KEY].as_str().unwrap_or_default();
			digits
				.parse::<num_bigint::BigInt>()
				.map(|big| Node::Number(Number::BigInt(Box::leak(Box::new(big)))))
				.unwrap_or_else(|_| Node::Text(digits.to_string()))
		}
		Value::Object(entries) if let Some(character) = codepoint_of(entries) => Node::Char(character),
		Value::Object(entries) => Node::List(
			entries.iter().map(|(key, value)| Node::key(key, node_of(value))).collect(),
			Bracket::Curly,
			Separator::Space,
		),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn json_conversion_keeps_nested_values_and_node_layout() {
		let value = serde_json::json!({
			"items": [1, 1.25, null],
			"enabled": true,
		});
		let expected = Node::List(
			vec![
				Node::key(
					"items",
					Node::List(vec![Node::int(1), Node::from(1.25), Node::Empty], Bracket::Square, Separator::Space),
				),
				Node::key("enabled", Node::int(1)),
			],
			Bracket::Curly,
			Separator::Space,
		);

		assert_eq!(node_of(&value), expected);
		assert_eq!(node_of(&serde_json::json!({ CODEPOINT_KEY: "🦀" })), Node::Char('🦀'));
	}
}
