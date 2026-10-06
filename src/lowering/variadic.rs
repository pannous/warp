//! Rest parameters and spread arguments, resolved at compile time so a call stays a plain call:
//! - `def total(xs...)`, `total(...xs)`, `total(*xs)` (Julia/Swift, JS, Python): the last parameter is the list of the
//!   arguments the others leave, `total(1, 2, 3)` → `total([1 2 3])`;
//! - `f(...xs)`, `f(xs...)`, `f(*xs)` spread a list: into a rest parameter it is passed as it is (`[2] + xs` after
//!   other leftover arguments), into fixed parameters as `xs#1, xs#2, …` up to the parameter count;
//! - `def f(x, **kw)` (Python): the last parameter is the object of the named arguments no other parameter takes,
//!   `f(5, a=1)` → `f(5, {a: 1})`;
//! - `f(**m)` spreads an object: into a `**kw` parameter it is the object, into fixed parameters the fields named like
//!   the parameters the other arguments leave, `g(1, **m)` → `g(1, b=m.b)`.

use crate::analyzer::call_name;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::tuples::STARRED;
use std::collections::HashMap;

/// A function's fixed parameters, whether a rest parameter follows them, and whether a keyword parameter `**kw` ends them
#[derive(Clone)]
struct Signature {
	fixed: Vec<String>,
	rest: bool,
	keywords: bool,
}

pub fn lower(node: Node) -> Node {
	let mut signatures = HashMap::new();
	collect_signatures(&node, &mut signatures);
	if !signatures.values().any(|signature| signature.rest || signature.keywords) && !has_spread(&node) {
		return node;
	}
	Variadic { signatures }.rewrite(node)
}

/// `*xs`, `...xs` (parsed as `*xs`) and `xs...` (an open range of xs): the name xs
fn starred(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => name.strip_prefix(STARRED).filter(|name| !name.is_empty() && !name.starts_with(STARRED)).map(str::to_string),
		Node::Key(name, Op::To, end) if matches!(end.drop_meta(), Node::Empty) => match name.drop_meta() {
			Node::Symbol(name) => Some(name.clone()),
			_ => None,
		},
		_ => None,
	}
}

/// `f(params) := body`: the head's items
fn definition_head(node: &Node) -> Option<&[Node]> {
	let Node::Key(head, Op::Define | Op::Assign, _) = node.drop_meta() else { return None };
	match head.drop_meta() {
		Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => Some(items),
		_ => None,
	}
}

/// `**kw`: the name kw
fn double_starred(node: &Node) -> Option<String> {
	match node.drop_meta() {
		Node::Symbol(name) => name.strip_prefix(STARRED)?.strip_prefix(STARRED).filter(|name| !name.is_empty()).map(str::to_string),
		_ => None,
	}
}

/// A parameter without its stars
fn unstarred(parameter: &Node) -> Node {
	double_starred(parameter).or_else(|| starred(parameter)).map(Node::Symbol).unwrap_or_else(|| parameter.clone())
}

fn collect_signatures(node: &Node, signatures: &mut HashMap<String, Signature>) {
	node.visit(&mut |part| {
		let Some(head) = definition_head(part) else { return };
		let mut parameters = &head[1..];
		let keywords = parameters.last().is_some_and(|last| double_starred(last).is_some());
		parameters = &parameters[..parameters.len() - keywords as usize];
		let rest = parameters.last().is_some_and(|last| starred(last).is_some());
		parameters = &parameters[..parameters.len() - rest as usize];
		let fixed = parameters.iter().map(|parameter| parameter_name(parameter).unwrap_or_default()).collect();
		signatures.insert(head[0].name(), Signature { fixed, rest, keywords });
	});
}

fn parameter_name(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, _, _) => parameter_name(name),
		_ => None,
	}
}

/// `a=1`, `a: 1` in a call: the name a and the value
fn named_argument(argument: &Node) -> Option<(String, Node)> {
	match argument.drop_meta() {
		Node::Key(name, Op::Assign | Op::Colon, value) => match name.drop_meta() {
			Node::Symbol(name) => Some((name.clone(), value.as_ref().clone())),
			_ => None,
		},
		_ => None,
	}
}

fn has_spread(node: &Node) -> bool {
	let mut found = false;
	node.visit(&mut |part| if let Node::List(items, bracket, separator) = part {
		found |= call_name(items, bracket, separator).is_some() && items[1..].iter().any(|item| starred(item).is_some() || double_starred(item).is_some());
	});
	found
}

struct Variadic {
	signatures: HashMap<String, Signature>,
}

impl Variadic {
	fn rewrite(&self, node: Node) -> Node {
		if let Some(head) = definition_head(&node) {
			let starred_parameter = head[1..].iter().any(|parameter| starred(parameter).is_some() || double_starred(parameter).is_some());
			let Node::Key(target, op, body) = node.drop_meta() else { unreachable!("a definition") };
			let target = if starred_parameter { Box::new(self.unstarred_head(target)) } else { target.clone() };
			return Node::Key(target, *op, Box::new(self.rewrite(body.as_ref().clone())));
		}
		match node {
			Node::List(items, bracket, separator) if self.is_call(&items, &bracket, &separator) => self.call(items),
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.rewrite(item)).collect(), bracket, separator),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}

	/// `(f a xs...)` → `(f a xs)`, `(f *args **kw)` → `(f args kw)`, also under a result type `f(a, xs...):int`
	fn unstarred_head(&self, target: &Node) -> Node {
		match target.drop_meta() {
			Node::Key(head, op, result) => Node::Key(Box::new(self.unstarred_head(head)), *op, result.clone()),
			Node::List(items, bracket, separator) => {
				let (name, parameters) = items.split_first().expect("a function name");
				Node::List(std::iter::once(name.clone()).chain(parameters.iter().map(unstarred)).collect(), bracket.clone(), separator.clone())
			}
			other => other.clone(),
		}
	}

	/// `f(…)` with a spread argument, a call of a function with a rest parameter, also `f 1 2 3`
	fn is_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
		let Some(name) = items.first().map(Node::drop_meta).and_then(|head| match head {
			Node::Symbol(name) => Some(name),
			_ => None,
		}) else { return false };
		let rest = self.signatures.get(name).is_some_and(|signature| signature.rest || signature.keywords);
		let spread = items[1..].iter().any(|item| starred(item).is_some() || double_starred(item).is_some());
		let called = call_name(items, bracket, separator).is_some() || (*separator == Separator::Space && matches!(bracket, Bracket::None | Bracket::Round));
		called && (rest || spread)
	}

	fn call(&self, items: Vec<Node>) -> Node {
		let mut items = items.into_iter();
		let head = items.next().expect("a function name");
		let arguments: Vec<Node> = items.map(|item| self.rewrite(item)).collect();
		let signature = self.signatures.get(&head.name());
		let (spread_objects, arguments): (Vec<Node>, Vec<Node>) = arguments.into_iter().partition(|argument| double_starred(argument).is_some());
		let spread_object = match (spread_objects.as_slice(), signature) {
			([], _) => None,
			([object], Some(_)) => double_starred(object),
			([_], None) => return crate::node::error(&format!("{}(**…) spreads an object into the parameters of a function defined in the program; {} is none", head.name(), head.name())),
			_ => return crate::node::error(&format!("{}(**a, **b): spread one object", head.name())),
		};
		let arguments = match (signature, &spread_object) {
			(Some(signature), Some(object)) if !signature.keywords => [arguments.clone(), fields_for_parameters(&arguments, &signature.fixed, object)].concat(),
			_ => arguments,
		};
		let (arguments, keywords) = match signature {
			Some(signature) if signature.keywords => match (keyword_object(arguments, &signature.fixed), spread_object) {
				((arguments, Some(Node::List(entries, _, _))), Some(object)) if entries.is_empty() => (arguments, Some(Node::Symbol(object))),
				(_, Some(_)) => return crate::node::error(&format!("{}(**m, k=v): the keyword object is m or the named arguments, not both yet", head.name())),
				(lowered, None) => lowered,
			},
			_ => (arguments, None),
		};
		let arguments = match signature {
			Some(Signature { fixed, rest: true, .. }) if arguments.len() >= fixed.len() => {
				let (fixed_arguments, leftover) = arguments.split_at(fixed.len());
				[spread_items(fixed_arguments, None), vec![rest_list(leftover)]].concat()
			}
			Some(Signature { fixed, rest: false, .. }) => spread_items(&arguments, Some(fixed.len())),
			_ => arguments.iter().map(|argument| starred(argument).map(Node::Symbol).unwrap_or_else(|| argument.clone())).collect(),
		};
		Node::List([vec![head], arguments, keywords.into_iter().collect()].concat(), Bracket::Round, Separator::None)
	}
}

/// `g(1, **m)` of `g(a, b, c)`: the named arguments `b=m.b, c=m.c` for the parameters the arguments leave
fn fields_for_parameters(arguments: &[Node], fixed: &[String], object: &str) -> Vec<Node> {
	let named: Vec<String> = arguments.iter().filter_map(named_argument).map(|(name, _)| name).collect();
	let positional = arguments.len() - named.len();
	fixed.iter().skip(positional).filter(|parameter| !named.contains(parameter)).map(|parameter| {
		let field = Node::Key(Box::new(Node::Symbol(object.to_string())), Op::Dot, Box::new(Node::Symbol(parameter.clone())));
		Node::Key(Box::new(Node::Symbol(parameter.clone())), Op::Assign, Box::new(field))
	}).collect()
}

/// The named arguments no fixed parameter takes, as the object of a `**kw` parameter, and the other arguments
fn keyword_object(arguments: Vec<Node>, fixed: &[String]) -> (Vec<Node>, Option<Node>) {
	let (named, others): (Vec<Node>, Vec<Node>) = arguments.into_iter().partition(|argument| named_argument(argument).is_some_and(|(name, _)| !fixed.contains(&name)));
	let entries: Vec<Node> = named.iter().filter_map(named_argument).map(|(name, value)| Node::Key(Box::new(Node::Symbol(name)), Op::Colon, Box::new(value))).collect();
	let separator = if entries.len() > 1 { Separator::Colon } else { Separator::None };
	(others, Some(Node::List(entries, Bracket::Curly, separator)))
}

/// The arguments with every spread list replaced by its items: a list literal's own items, else `xs#1, xs#2, …` as many
/// as the `count` of parameters leaves for it
fn spread_items(arguments: &[Node], count: Option<usize>) -> Vec<Node> {
	let mut items = vec![];
	for (index, argument) in arguments.iter().enumerate() {
		match (starred(argument), argument.drop_meta()) {
			(Some(name), _) => {
				let left_for_it = count.unwrap_or(0).saturating_sub(items.len() + arguments.len() - index - 1);
				items.extend((1..=left_for_it).map(|position| Node::Key(Box::new(Node::Symbol(name.clone())), Op::Hash, Box::new(crate::node::int(position as i64)))));
			}
			_ => items.push(argument.clone()),
		}
	}
	items
}

/// The leftover arguments of a rest parameter as one list: `[8 9]`, a lone spread `xs` as it is, `[2] + xs` after others
fn rest_list(leftover: &[Node]) -> Node {
	let mut list: Option<Node> = None;
	let mut pending: Vec<Node> = vec![];
	let append = |list: Option<Node>, part: Node| match list {
		Some(list) => Some(Node::Key(Box::new(list), Op::Add, Box::new(part))),
		None => Some(part),
	};
	for argument in leftover {
		match starred(argument) {
			Some(name) => {
				if !pending.is_empty() {
					list = append(list, Node::List(std::mem::take(&mut pending), Bracket::Square, Separator::Space));
				}
				list = append(list, Node::Symbol(name));
			}
			None => pending.push(argument.clone()),
		}
	}
	if !pending.is_empty() || list.is_none() {
		list = append(list, Node::List(pending, Bracket::Square, Separator::Space));
	}
	list.expect("a list")
}
