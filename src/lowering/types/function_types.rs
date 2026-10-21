//! A parameter typed as a function, `signal: (float) -> float` (card parameter-typed): a function passed for it whose
//! parameters are untyped takes the declared parameter types, its value cast to the declared result, so
//! `hum(t) := sin(2 * pi * 440 * t); play_signal(hum, 2)` hands the host a function of a float. A parameter passed on
//! to such a parameter is one too: `play(tone, duration) := … play_signal(tone, duration)` types the functions played.
//! A list literal of functions given for it, the stereo pair `[left, right]`, types each. Definitions of one name and
//! parameter count may declare different function types at one place, `play(fill: (samples, float) -> any, duration)`
//! next to `play(tone, duration)`: a function passed there takes the one with as many parameters as it has.

use super::nodes::{function_head, key};
use crate::node::{parens, Bracket, Node};
use crate::operators::Op;
use std::collections::HashMap;

/// `(float) -> float`: the parameter types and the result type a function-typed parameter declares
#[derive(Clone, PartialEq)]
struct Signature {
	parameters: Vec<Node>,
	result: Node,
	/// declared by one of several definitions of a name and arity, which overloads.rs picks by the passed function's
	/// parameters, only for a function defined once
	of_overload: bool,
}

/// A definition's name and parameter count, which tell its overloads apart: `play(tone)`, `play(tone, duration)`
type Arity = (String, usize);
/// The function types the definitions of a name and arity declare at each place, the alternatives in declaration order
type Signatures = HashMap<Arity, Vec<Vec<Signature>>>;

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

/// The function-typed parameters the definitions declare, by their places
fn declared_signatures(statements: &[&Node]) -> Signatures {
	let mut signatures = Signatures::new();
	let heads: Vec<_> = statements.iter().filter_map(|statement| function_head(statement)).collect();
	let mut definitions: HashMap<Arity, usize> = HashMap::new();
	for head in &heads {
		*definitions.entry((head.name.to_string(), head.parameters().len())).or_default() += 1;
	}
	for head in &heads {
		let parameters = head.parameters();
		let of_overload = definitions[&(head.name.to_string(), parameters.len())] > 1;
		for (index, parameter) in parameters.iter().enumerate() {
			if let Some(signature) =
				declared_signature(parameter).map(|signature| Signature { of_overload, ..signature })
			{
				add_alternative(&mut signatures, (head.name.to_string(), parameters.len()), index, signature);
			}
		}
	}
	signatures
}

/// Whether the signature is new at that place
fn add_alternative(signatures: &mut Signatures, arity: Arity, index: usize, signature: Signature) -> bool {
	let places = signatures.entry(arity.clone()).or_insert_with(|| vec![vec![]; arity.1]);
	let alternatives = &mut places[index];
	let new = !alternatives.contains(&signature);
	if new {
		alternatives.push(signature);
	}
	new
}

/// How many arguments a parameter typed as a function takes: 1 of `f: (float) -> float`
pub(crate) fn declared_arity(parameter: &Node) -> Option<usize> {
	declared_signature(parameter).map(|signature| signature.parameters.len())
}

/// The parameter types of a parameter typed as a function: float of `f: (float) -> float`
pub(crate) fn declared_parameter_types(parameter: &Node) -> Option<Vec<String>> {
	declared_signature(parameter).map(|signature| signature.parameters.iter().map(Node::name).collect())
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
	Some(Signature { parameters, result: (**result).clone(), of_overload: false })
}

/// An untyped parameter passed on to a function-typed parameter takes its signatures, until no more do
fn propagated(mut signatures: Signatures, statements: &[&Node]) -> Signatures {
	loop {
		let mut changed = false;
		for statement in statements {
			let Some(head) = function_head(statement) else { continue };
			let parameters = head.parameters();
			let arity = (head.name.to_string(), parameters.len());
			for (index, parameter) in parameters.iter().enumerate() {
				let Some(name) = parameter.symbol_name() else { continue };
				for signature in passed_signatures(head.body, name, &signatures) {
					changed |= add_alternative(&mut signatures, arity.clone(), index, signature);
				}
			}
		}
		if !changed {
			return signatures;
		}
	}
}

/// The signatures of the function-typed parameters the body passes `name` to
fn passed_signatures(body: &Node, name: &str, signatures: &Signatures) -> Vec<Signature> {
	let mut found = vec![];
	visit_typed_arguments(body, signatures, &mut |argument, signature| {
		if argument.is_symbol(name) {
			found.push(Signature { of_overload: false, ..signature.clone() });
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
		for (argument, alternatives) in arguments.iter().zip(places) {
			for signature in alternatives {
				match argument.drop_meta() {
					Node::List(items, Bracket::Square, _) => items.iter().for_each(|item| action(item, signature)),
					single => action(single, signature),
				}
			}
		}
	});
}

/// The functions the program passes for function-typed parameters, by name, with the signatures wanted there; not a
/// definition's own parameters, as `tone` of `play(tone, duration)`, which the library's function `tone` does not answer
fn passed_functions(statements: &[&Node], signatures: &Signatures) -> HashMap<String, Vec<Signature>> {
	let mut passed = HashMap::new();
	let mut definitions: HashMap<&str, usize> = HashMap::new();
	for head in statements.iter().filter_map(|statement| function_head(statement)) {
		*definitions.entry(head.name).or_default() += 1;
	}
	// `play(chord, 2s)` of a program's chord(t) beside sound's chord(root, kind): no overload picks either by its
	// parameters, so neither takes such a type
	let is_ambiguous = |name: &str| definitions.get(name).is_some_and(|count| *count > 1);
	for statement in statements {
		let own: Vec<&str> = function_head(statement)
			.map(|head| head.parameters().iter().filter_map(|parameter| parameter.symbol_name()).collect())
			.unwrap_or_default();
		visit_typed_arguments(statement, signatures, &mut |argument, signature| {
			let name = argument.symbol_name().filter(|name| !own.contains(name));
			if let Some(name) = name.filter(|name| !(signature.of_overload && is_ambiguous(name))) {
				let wanted: &mut Vec<Signature> = passed.entry(name.to_string()).or_default();
				if !wanted.contains(signature) {
					wanted.push(signature.clone());
				}
			}
		});
	}
	passed
}

fn typed_definitions(program: Node, passed: &HashMap<String, Vec<Signature>>) -> Node {
	match program {
		Node::List(items, bracket, separator) if separator.separates_statements() => Node::List(
			items.into_iter().map(|statement| typed_definition(statement, passed)).collect(),
			bracket,
			separator,
		),
		statement => typed_definition(statement, passed),
	}
}

/// `hum(t) := body` passed for `(float) -> float`: `hum(t: float) := (body) as float`, by the signature with as many
/// parameters as hum, a typed one kept as declared
fn typed_definition(statement: Node, passed: &HashMap<String, Vec<Signature>>) -> Node {
	let Some((types, result)) = function_head(&statement).and_then(|head| {
		let parameters = head.parameters();
		let signature = passed
			.get(head.name)
			.filter(|_| head.is_untyped_call() && !parameters.is_empty())?
			.iter()
			.find(|signature| signature.parameters.len() == parameters.len())?;
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
	// `-> any`: whatever the body gives
	let body = match result.is_symbol(crate::words::ANY_TYPE) {
		true => body,
		false => key(parens(vec![body]), Op::As, result),
	};
	rebuild(typed_head(head, &types), body)
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
