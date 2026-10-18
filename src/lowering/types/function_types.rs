//! A parameter typed as a function, `signal: (float) -> float` (card parameter-typed): a function passed for it whose
//! parameters are untyped takes the declared parameter types, its value cast to the declared result, so
//! `hum(t) := sin(2 * pi * 440 * t); play_signal(hum, 2)` hands the host a function of a float. A parameter passed on
//! to such a parameter is one too: `play(tone, duration) := … play_signal(tone, duration)` types the functions played.
//! A list literal of functions given for it, the stereo pair `[left, right]`, types each.

use super::nodes::{function_head, key};
use crate::node::{parens, Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;

/// `(float) -> float`: the parameter types and the result type a function-typed parameter declares
#[derive(Clone, PartialEq)]
struct Signature {
	parameters: Vec<Node>,
	result: Node,
}

/// A definition's name and parameter count, which tell its overloads apart: `play(tone)`, `play(tone, duration)`
type Arity = (String, usize);
/// The function-typed parameters of each definition, by their places
type Signatures = HashMap<Arity, Vec<Option<Signature>>>;

pub fn lower(program: Node) -> Node {
	let statements = top_level(&program);
	let signatures = propagated(declared_signatures(&statements), &statements);
	if signatures.is_empty() {
		return program;
	}
	let passed = passed_functions(&statements, &signatures);
	if passed.is_empty() {
		return program;
	}
	typed_definitions(program, &passed)
}

fn top_level(program: &Node) -> Vec<&Node> {
	match program.drop_meta() {
		Node::List(items, _, separator) if separator.separates_statements() => items.iter().collect(),
		statement => vec![statement],
	}
}

/// The function-typed parameters each definition declares, by their places
fn declared_signatures(statements: &[&Node]) -> Signatures {
	statements
		.iter()
		.filter_map(|statement| {
			let head = function_head(statement)?;
			let parameters = head.parameters();
			let signatures: Vec<_> = parameters.iter().map(|parameter| declared_signature(parameter)).collect();
			signatures.iter().any(Option::is_some).then(|| ((head.name.to_string(), parameters.len()), signatures))
		})
		.collect()
}

/// `f: (float) -> float`, parsed `(f: (float)) -> float`
fn declared_signature(parameter: &Node) -> Option<Signature> {
	let Node::Key(typed, Op::Arrow, result) = parameter.drop_meta() else {
		return None;
	};
	let Node::Key(_, Op::Colon, parameters) = typed.drop_meta() else {
		return None;
	};
	let parameters = match parameters.drop_meta() {
		Node::List(types, Bracket::Round, _) => types.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	};
	Some(Signature { parameters, result: (**result).clone() })
}

/// An untyped parameter passed on to a function-typed parameter takes its signature, until no more do
fn propagated(mut signatures: Signatures, statements: &[&Node]) -> Signatures {
	loop {
		let mut changed = false;
		for statement in statements {
			let Some(head) = function_head(statement) else { continue };
			let parameters = head.parameters();
			let arity = (head.name.to_string(), parameters.len());
			for (index, parameter) in parameters.iter().enumerate() {
				let Some(name) = parameter.symbol_name() else { continue };
				let known = signatures.get(&arity).and_then(|places| places[index].as_ref());
				if known.is_some() {
					continue;
				}
				if let Some(signature) = passed_signature(head.body, name, &signatures) {
					let places = signatures.entry(arity.clone()).or_insert_with(|| vec![None; parameters.len()]);
					places[index] = Some(signature);
					changed = true;
				}
			}
		}
		if !changed {
			return signatures;
		}
	}
}

/// The signature of the first function-typed parameter the body passes `name` to
fn passed_signature(body: &Node, name: &str, signatures: &Signatures) -> Option<Signature> {
	let mut found = None;
	visit_typed_arguments(body, signatures, &mut |argument, signature| {
		if found.is_none() && argument.is_symbol(name) {
			found = Some(signature.clone());
		}
	});
	found
}

/// Each argument of a call given for a function-typed parameter, the items of a list literal one by one
fn visit_typed_arguments(node: &Node, signatures: &Signatures, action: &mut dyn FnMut(&Node, &Signature)) {
	node.visit(&mut |part| {
		let Some((callee, arguments)) = part.call_parts() else { return };
		let arguments = crate::analyzer::effects::call_arguments(arguments);
		let Some(places) = signatures.get(&(callee.to_string(), arguments.len())) else { return };
		for (argument, signature) in arguments.iter().zip(places) {
			let Some(signature) = signature else { continue };
			match argument.drop_meta() {
				Node::List(items, Bracket::Square, _) => items.iter().for_each(|item| action(item, signature)),
				single => action(single, signature),
			}
		}
	});
}

/// The functions the program passes for function-typed parameters, by name; not a definition's own parameters, as
/// `tone` of `play(tone, duration)`, which the library's function `tone` does not answer
fn passed_functions(statements: &[&Node], signatures: &Signatures) -> HashMap<String, Signature> {
	let mut passed = HashMap::new();
	for statement in statements {
		let own: Vec<&str> = function_head(statement)
			.map(|head| head.parameters().iter().filter_map(|parameter| parameter.symbol_name()).collect())
			.unwrap_or_default();
		visit_typed_arguments(statement, signatures, &mut |argument, signature| {
			if let Some(name) = argument.symbol_name().filter(|name| !own.contains(name)) {
				passed.entry(name.to_string()).or_insert_with(|| signature.clone());
			}
		});
	}
	passed
}

fn typed_definitions(program: Node, passed: &HashMap<String, Signature>) -> Node {
	match program {
		Node::List(items, bracket, separator) if separator.separates_statements() => Node::List(
			items.into_iter().map(|statement| typed_definition(statement, passed)).collect(),
			bracket,
			separator,
		),
		statement => typed_definition(statement, passed),
	}
}

/// `hum(t) := body` passed for `(float) -> float`: `hum(t: float) := (body) as float`, with as many parameters as the
/// signature, a typed one kept as declared
fn typed_definition(statement: Node, passed: &HashMap<String, Signature>) -> Node {
	let Some((types, result)) = function_head(&statement).and_then(|head| {
		let signature = passed.get(head.name).filter(|_| head.is_untyped_call())?;
		let parameters = head.parameters();
		(!parameters.is_empty() && parameters.len() == signature.parameters.len()).then_some(())?;
		let untyped = parameters.iter().zip(&signature.parameters).filter_map(|(parameter, declared)| {
			parameter.symbol_name().map(|name| (name.to_string(), declared.clone()))
		});
		Some((untyped.collect::<HashMap<_, _>>(), signature.result.clone()))
	}) else {
		return statement;
	};
	let Some((head, body, rebuild)) = crate::lowering::memoization::definition_parts(&statement) else {
		return statement;
	};
	rebuild(typed_head(head, &types), key(parens(vec![body]), Op::As, result))
}

/// The head with each named parameter typed: `(hum t)` → `(hum (: t float))`, the name first left as it is
fn typed_head(head: Node, types: &HashMap<String, Node>) -> Node {
	match head {
		Node::List(items, bracket, separator) => {
			let mut items = items.into_iter();
			let name = items.next();
			Node::List(
				name.into_iter().chain(items.map(|item| typed_parameters(item, types))).collect(),
				bracket,
				separator,
			)
		}
		Node::Meta { node, data } => Node::Meta { node: Box::new(typed_head(*node, types)), data },
		other => other,
	}
}

fn typed_parameters(node: Node, types: &HashMap<String, Node>) -> Node {
	match node.symbol_name().and_then(|name| types.get(name)) {
		Some(declared) => key(node, Op::Colon, declared.clone()),
		None => node.map_children(|child| typed_parameters(child, types)),
	}
}
