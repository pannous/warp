//! Callable (wiki trait.md): an instance applied to arguments is the `call` its type defines, `room(signal)` and
//! `signal |> room` of a Bus room are `call(room, signal)` (lib/sound.warp), wherever the instance comes from: a variable,
//! a parameter, a function's result, an expression, a field (`m.room`), a list's item or a lambda's parameter. Where the
//! compiler cannot know the type (an untyped parameter, a field or item of a value of unknown type) the call dispatches
//! at run time: an instance of a type defining `call` is called through it, any other value as a function value, which
//! says loudly when it is none. Before the closure passes read `room(signal)` as a call of a function value
use super::*;
use crate::words::INSTANCE_OF;

pub fn lower_instance_calls(node: Node) -> Node {
	let types = InstanceTypes::of(&node);
	if types.registry.types().is_empty() {
		return node;
	}
	let traits = Traits::of(&node);
	let mut calls = InstanceCalls {
		signatures: instance_signatures(&node, &types.registry, &traits),
		functions: defined_functions(&node),
		types,
		callable: vec![],
		untyped: RefCell::default(),
	};
	let type_names: Vec<String> = calls.types.registry.types().iter().map(|type_def| type_def.name.clone()).collect();
	calls.callable = type_names.into_iter().filter(|type_name| calls.defines_call(type_name)).collect();
	if calls.callable.is_empty() {
		return node;
	}
	calls.rewrite(node)
}

struct InstanceCalls {
	types: InstanceTypes,
	signatures: HashMap<String, Vec<(String, Option<String>)>>,
	functions: Vec<String>,
	/// The types defining `call`
	callable: Vec<String>,
	/// The untyped parameters of the definitions and lambdas being rewritten, innermost last
	untyped: RefCell<Vec<Vec<String>>>,
}

impl InstanceCalls {
	fn rewrite(&self, node: Node) -> Node {
		match node {
			// a definition head names parameters, it calls nothing; its body sees them typed
			Node::Key(head, op @ (Op::Assign | Op::Define), body) if definition_head(&head).is_some() => {
				self.untyped.borrow_mut().push(untyped_parameters(&head));
				let body = self.types.in_definition(&head, || self.rewrite(*body));
				self.untyped.borrow_mut().pop();
				Node::Key(head, op, Box::new(body))
			}
			// `d => d(5)`, `(a, d) => a + d(1)`: a lambda's parameters are as untyped as a definition's
			Node::Key(parameters, Op::Arrow, body) => {
				self.untyped.borrow_mut().push(lambda_parameters(&parameters));
				let body = self.rewrite(*body);
				self.untyped.borrow_mut().pop();
				Node::Key(parameters, Op::Arrow, Box::new(body))
			}
			// `m.room(signal)`: the field room applied, not a method of m
			Node::Key(receiver, Op::Dot, member) => {
				let receiver = self.rewrite(*receiver);
				// the member's own word is no call head: its arguments are rewritten, not it
				let member = match *member {
					Node::List(items, bracket, separator) => Node::List(
						items
							.into_iter()
							.enumerate()
							.map(|(index, item)| if index == 0 { item } else { self.rewrite(item) })
							.collect(),
						bracket,
						separator,
					),
					other => self.rewrite(other),
				};
				match member.drop_meta() {
					Node::List(items, Bracket::Round, Separator::None | Separator::Space) if items.len() > 1 => {
						let field = key(receiver.clone(), Op::Dot, items[0].clone());
						match self.types.shape(&field) {
							Some(Shape::Instance(type_name)) if self.callable.contains(&type_name) => {
								parens([vec![symbol(CALL), field], items[1..].to_vec()].concat())
							}
							_ => key(receiver, Op::Dot, member),
						}
					}
					_ => key(receiver, Op::Dot, member),
				}
			}
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.rewrite(item)).collect();
				// a call, `room(signal)`, `signal |> room`: not a list `[a, b]`, a tuple `(a, b)` or the words of a
				// phrase, `room in rooms`
				let is_call = bracket == Bracket::Round && matches!(separator, Separator::None | Separator::Space);
				match is_call.then(|| self.called(&items)).flatten().or_else(|| self.result_called(&items, &bracket)) {
					Some(called) => called,
					None => Node::List(items, bracket, separator),
				}
			}
			other => children_rewritten(other, |child| self.rewrite(child)),
		}
	}

	/// `room(signal)` of an instance whose type defines `call`: `call(room, signal)`; of a value whose type is known only
	/// at run time, the dispatch on it
	fn called(&self, items: &[Node]) -> Option<Node> {
		let [head, arguments @ ..] = items else {
			return None;
		};
		let is_function = matches!(head.drop_meta(), Node::Symbol(name) if self.functions.contains(name));
		if arguments.is_empty() || is_function {
			return None;
		}
		match self.types.shape(head) {
			Some(Shape::Instance(type_name)) => {
				self.callable.contains(&type_name).then(|| self.call_of(head, arguments))
			}
			Some(Shape::ListOf(_)) => None,
			None => self.is_unknown(head).then(|| self.dispatched(head, arguments)),
		}
	}

	/// `mk()(2)`, the call's result applied (parsed as the call and its arguments juxtaposed): `call(mk(), 2)` of an
	/// instance, the dispatch on the result where its shape is unknown
	fn result_called(&self, items: &[Node], bracket: &Bracket) -> Option<Node> {
		let [head, Node::List(arguments, Bracket::Round, _)] = items else {
			return None;
		};
		let is_call = matches!(head.drop_meta(), Node::List(called, Bracket::Round, Separator::None)
			if called.first().and_then(Node::symbol_name).is_some_and(|name| self.functions.iter().any(|function| function == name)));
		if *bracket == Bracket::Square || !is_call || arguments.is_empty() {
			return None;
		}
		match self.types.shape(head) {
			Some(Shape::Instance(type_name)) => {
				self.callable.contains(&type_name).then(|| self.call_of(head, arguments))
			}
			Some(Shape::ListOf(_)) => None,
			None => Some(self.dispatched(head, arguments)),
		}
	}

	/// A head of a type known only at run time: an untyped parameter of a definition or lambda, a field or item of a
	/// value of unknown type
	fn is_unknown(&self, head: &Node) -> bool {
		match head.drop_meta() {
			Node::Symbol(name) => self.untyped.borrow().iter().flatten().any(|parameter| parameter == name),
			Node::Key(_, Op::Dot | Op::Hash, _) => true,
			_ => false,
		}
	}

	fn call_of(&self, head: &Node, arguments: &[Node]) -> Node {
		parens([vec![symbol(CALL), head.clone()], arguments.to_vec()].concat())
	}

	/// `if instance_of(b, "Bus") then call(b, signal) else b(signal)`, a case per type defining `call`
	/// The head is evaluated once: `mk()(2)` binds `callee·1 = mk()` first
	fn dispatched(&self, head: &Node, arguments: &[Node]) -> Node {
		let mut bindings = vec![];
		let head = match head.drop_meta() {
			Node::Symbol(_) => head.clone(),
			_ => crate::lowering::nodes::bound_once("callee", head.clone(), &mut bindings),
		};
		let as_function = parens([vec![head.clone()], arguments.to_vec()].concat());
		// `f(x, i)` of a library's untyped f is no `call(b: Bus, signal)`: only a call taking these arguments is tried
		let takes_these =
			|type_name: &&String| self.call_arity(type_name).is_none_or(|arity| arity == arguments.len() + 1);
		let cases = self.callable.iter().filter(takes_these).map(|type_name| {
			let test = call(INSTANCE_OF, vec![head.clone(), Node::Text(type_name.clone())]);
			(Some(test), self.call_of(&head, arguments))
		});
		let dispatch = crate::lowering::nodes::tested_cases(cases.collect(), as_function);
		crate::lowering::nodes::after_bindings(bindings, dispatch)
	}

	/// The parameter count of the `call` of type_name, the instance included: 2 of `call(b: Bus, signal)`
	fn call_arity(&self, type_name: &str) -> Option<usize> {
		self.signatures.get(&witness_name(CALL, type_name)).or_else(|| self.signatures.get(CALL)).map(Vec::len)
	}

	/// `call(b: Bus, signal)`: one definition keeps its name, several of the operation are its witnesses `call·Bus`
	fn defines_call(&self, type_name: &str) -> bool {
		let first_declared = |(_, declared): &(String, Option<String>)| declared.as_deref() == Some(type_name);
		self.functions.contains(&witness_name(CALL, type_name))
			|| self.signatures.get(CALL).and_then(|parameters| parameters.first()).is_some_and(first_declared)
	}
}

/// `through(b, gain = 1)`: b and gain, not the typed `s: Bus` or `n: int`
/// `d => …`, `(a, d) => …`: a and d
fn lambda_parameters(parameters: &Node) -> Vec<String> {
	match parameters.drop_meta() {
		Node::Symbol(name) => vec![name.clone()],
		Node::List(items, _, _) => items.iter().flat_map(lambda_parameters).collect(),
		_ => vec![],
	}
}
