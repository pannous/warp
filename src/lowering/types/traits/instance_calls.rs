//! Callable (wiki trait.md): an instance applied to arguments is the `call` its type defines, `room(signal)` and
//! `signal |> room` of a Bus room are `call(room, signal)` (lib/sound.warp), wherever the instance comes from: a variable,
//! a parameter, a function's result or an expression. Before the closure passes read `room(signal)` as a call of a
//! function value
use super::*;

pub fn lower_instance_calls(node: Node) -> Node {
	let types = InstanceTypes::of(&node);
	if types.registry.types().is_empty() {
		return node;
	}
	let traits = Traits::of(&node);
	let calls = InstanceCalls {
		signatures: instance_signatures(&node, &types.registry, &traits),
		functions: defined_functions(&node),
		types,
	};
	if !calls.signatures.contains_key(CALL) && !calls.functions.iter().any(|name| witness_operation(name) == CALL) {
		return node;
	}
	calls.rewrite(node)
}

struct InstanceCalls {
	types: InstanceTypes,
	signatures: HashMap<String, Vec<(String, Option<String>)>>,
	functions: Vec<String>,
}

impl InstanceCalls {
	fn rewrite(&self, node: Node) -> Node {
		match node {
			// a definition head names parameters, it calls nothing; its body sees them typed
			Node::Key(head, op @ (Op::Assign | Op::Define), body) if definition_head(&head).is_some() => {
				let body = self.types.in_definition(&head, || self.rewrite(*body));
				Node::Key(head, op, Box::new(body))
			}
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.rewrite(item)).collect();
				match self.called(&items) {
					Some(called) => parens(called),
					None => Node::List(items, bracket, separator),
				}
			}
			other => children_rewritten(other, |child| self.rewrite(child)),
		}
	}

	/// `room(signal)` of an instance whose type defines `call`: `call(room, signal)`
	fn called(&self, items: &[Node]) -> Option<Vec<Node>> {
		let [head, arguments @ ..] = items else {
			return None;
		};
		let is_function = matches!(head.drop_meta(), Node::Symbol(name) if self.functions.contains(name));
		if arguments.is_empty() || is_function {
			return None;
		}
		let Some(Shape::Instance(type_name)) = self.types.shape(head) else {
			return None;
		};
		self.defines_call(&type_name).then(|| [vec![symbol(CALL), head.clone()], arguments.to_vec()].concat())
	}

	/// `call(b: Bus, signal)`: one definition keeps its name, several of the operation are its witnesses `call·Bus`
	fn defines_call(&self, type_name: &str) -> bool {
		let first_declared = |(_, declared): &(String, Option<String>)| declared.as_deref() == Some(type_name);
		self.functions.contains(&witness_name(CALL, type_name))
			|| self.signatures.get(CALL).and_then(|parameters| parameters.first()).is_some_and(first_declared)
	}
}
