//! D10 "Dispatch on return type" (notes/dispatch.md): definitions of one name with different result types are overloads,
//! renamed `render·pdf`, `render·docx` in declaration order. A call picks the overload its expected result type names
//! (`render "x" as pdf`, `docx d = render "x"`, `d:docx = …`, an argument of a parameter `d:docx`); without one the
//! first-declared overload is taken with a got-it warning naming the explicit form.

use crate::analyzer::{call_name, collect_all_types, type_word_kind};
use crate::diagnostic::{ask, reading, Ask, Fallback};
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::traits::{witness_name, InstanceTypes, Shape};
use crate::type_kinds::{Kind, TypeRegistry};
use std::collections::HashMap;

const TOPIC: &str = "return-type";

/// A definition `name(params) := body`, `name(params):T := body` or `name(params) as T := body`
struct Definition<'a> {
	head: &'a [Node],
	written_result: Option<String>,
	body: &'a Node,
}

fn definition(node: &Node) -> Option<Definition<'_>> {
	let Node::Key(target, Op::Assign | Op::Define, body) = node.drop_meta() else { return None };
	let (head, written_result) = match target.drop_meta() {
		Node::Key(head, Op::Colon | Op::As, result) => match result.drop_meta() {
			Node::Symbol(result) => (head.drop_meta(), Some(result.clone())),
			_ => return None,
		},
		head => (head, None),
	};
	match head {
		Node::List(items, Bracket::Round, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) => Some(Definition { head: items, written_result, body }),
		_ => None,
	}
}

pub fn lower(node: Node) -> Node {
	let node = lower_parameter_overloads(node);
	let mut registry = TypeRegistry::new();
	collect_all_types(&mut registry, &node);
	let mut lowering = Lowering { registry, types: InstanceTypes::of(&node), overloads: HashMap::new(), parameter_types: HashMap::new() };
	let mut results: Vec<(String, Option<String>)> = vec![];
	lowering.collect(&node, &mut results);
	lowering.overloads = overloads(&results);
	// definitions that differ in a parameter type (methods of two classes) dispatch on it (traits.rs), not on their result
	let mut signatures = HashMap::new();
	collect_signatures(&node, &mut signatures);
	lowering.overloads.retain(|name, _| signatures.get(name).is_none_or(|variants| variants.iter().all(|types| *types == variants[0])));
	if lowering.overloads.is_empty() && !results.iter().any(|(_, result)| result.as_deref().is_some_and(|result| lowering.is_declared(result))) {
		return node;
	}
	lowering.rewrite(node, None)
}

/// `f(x)`, `f x`, `(f x)`
fn is_call(items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
	call_name(items, bracket, separator).is_some() || (*separator == Separator::Space && matches!(bracket, Bracket::None | Bracket::Round))
}

fn parameter_type(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		Node::Key(_, Op::Colon, type_node) => Some(type_node.drop_meta().name()),
		_ => None,
	}
}

/// Names defined more than once, every time with a known and different result type: their result types in order
fn overloads(results: &[(String, Option<String>)]) -> HashMap<String, Vec<String>> {
	let mut by_name: HashMap<String, Vec<Option<String>>> = HashMap::new();
	for (name, result) in results {
		by_name.entry(name.clone()).or_default().push(result.clone());
	}
	by_name
		.into_iter()
		.filter(|(_, results)| results.len() > 1)
		.filter_map(|(name, results)| {
			let results: Vec<String> = results.into_iter().collect::<Option<_>>()?;
			let distinct = results.iter().enumerate().all(|(index, result)| !results[..index].contains(result));
			distinct.then_some((name, results))
		})
		.collect()
}

struct Lowering {
	registry: TypeRegistry,
	types: InstanceTypes,
	overloads: HashMap<String, Vec<String>>,
	parameter_types: HashMap<String, Vec<Option<String>>>,
}

impl Lowering {
	/// The result type of every definition in order, and the declared parameter types of every function
	fn collect(&mut self, node: &Node, results: &mut Vec<(String, Option<String>)>) {
		if let Some(definition) = definition(node) {
			let name = definition.head[0].name();
			results.push((name.clone(), self.result_type(&definition)));
			self.parameter_types.insert(name, definition.head[1..].iter().map(parameter_type).collect());
			return self.collect(definition.body, results);
		}
		match node.drop_meta() {
			Node::Key(left, _, right) => {
				self.collect(left, results);
				self.collect(right, results);
			}
			Node::List(items, _, _) => items.iter().for_each(|item| self.collect(item, results)),
			_ => {}
		}
	}

	fn is_declared(&self, type_name: &str) -> bool {
		self.registry.get_by_name(type_name).is_some()
	}

	fn is_type(&self, word: &str) -> bool {
		self.is_declared(word) || type_word_kind(word).is_some()
	}

	/// The overloaded function a call names, with its arguments: `render "x"`, `render("x")`
	fn overloaded_call<'a>(&self, node: &'a Node) -> Option<(&'a Node, &'a [Node])> {
		let Node::List(items, bracket, separator) = node.drop_meta() else { return None };
		if let ([single], Bracket::Round) = (items.as_slice(), bracket) {
			return self.overloaded_call(single);
		}
		let [head, arguments @ ..] = items.as_slice() else { return None };
		(is_call(items, bracket, separator) && !arguments.is_empty() && self.overloads.contains_key(&head.name())).then_some((head, arguments))
	}

	fn rewrite(&self, node: Node, expected: Option<&str>) -> Node {
		if let Some(rewritten) = self.rewrite_definition(&node) {
			return rewritten;
		}
		if let Some((head, arguments)) = self.overloaded_call(&node) {
			return self.resolve(&node, head, arguments, expected);
		}
		match node {
			// `render "x" as pdf` reads as `render ("x" as pdf)`: the `as` names the result of an overloaded call
			Node::List(items, Bracket::None, Separator::Space) if self.as_result(&items).is_some() => {
				let (head, argument, result) = self.as_result(&items).expect("checked");
				let argument = self.rewrite(argument.clone(), None);
				self.call(head, &result, vec![argument])
			}
			// `(render "x") as pdf`
			Node::Key(value, Op::As, result) if self.picks(&value, &result.drop_meta().name()) => self.rewrite(*value, Some(&result.drop_meta().name())),
			// `d:docx = render "x"`
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
				let declared = match target.drop_meta() {
					Node::Key(_, Op::Colon, type_node) => Some(type_node.drop_meta().name()),
					_ => None,
				};
				Node::Key(target, op, Box::new(self.rewrite(*value, declared.as_deref())))
			}
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left, None)), op, Box::new(self.rewrite(*right, None))),
			// `docx d = render "x"`; of a declared type it is the assignment, the value has the type
			Node::List(items, Bracket::None, Separator::Space) if self.typed_declaration(&items).is_some() => {
				let declared = self.typed_declaration(&items).expect("checked");
				let mut items = items.into_iter();
				let type_word = items.next().expect("checked");
				let Some(Node::Key(target, op, value)) = items.next().map(|declaration| declaration.drop_meta().clone()) else { unreachable!("checked") };
				let declaration = Node::Key(target, op, Box::new(self.rewrite(*value, Some(&declared))));
				if self.is_declared(&declared) { declaration } else { Node::List(vec![type_word, declaration], Bracket::None, Separator::Space) }
			}
			// `(render "x")` passes the expected type on
			Node::List(items, Bracket::Round, separator) if items.len() == 1 => {
				let item = items.into_iter().next().expect("one item");
				Node::List(vec![self.rewrite(item, expected)], Bracket::Round, separator)
			}
			Node::List(items, bracket, separator) => {
				let expected_arguments = self.argument_types(&items, &bracket, &separator);
				let items = items.into_iter().enumerate().map(|(index, item)| {
					let expected = expected_arguments.as_ref().and_then(|types| types.get(index.wrapping_sub(1)).cloned().flatten());
					self.rewrite(item, expected.as_deref())
				});
				Node::List(items.collect(), bracket, separator)
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node, expected)), data },
			other => other,
		}
	}

	/// A definition: an overload renamed to its variant, a result type naming a declared type dropped from the head (the
	/// body constructs it); its body rewritten
	fn rewrite_definition(&self, node: &Node) -> Option<Node> {
		let definition = definition(node)?;
		let Node::Key(target, op, _) = node.drop_meta() else { return None };
		let name = definition.head[0].name();
		let body = Box::new(self.rewrite(definition.body.clone(), None));
		let variant = self.overloads.contains_key(&name).then(|| self.result_type(&definition)).flatten().map(|result| witness_name(&name, &result));
		let drops_result = definition.written_result.as_deref().is_some_and(|result| self.is_declared(result));
		if variant.is_none() && !drops_result {
			return Some(Node::Key(target.clone(), *op, body));
		}
		let mut head = definition.head.to_vec();
		if let Some(variant) = variant {
			head[0] = Node::Symbol(variant);
		}
		Some(Node::Key(Box::new(Node::List(head, Bracket::Round, Separator::None)), *op, body))
	}

	/// The written result type, else the type of the instance the body constructs
	fn result_type(&self, definition: &Definition) -> Option<String> {
		definition.written_result.clone().or_else(|| match self.types.shape(definition.body) {
			Some(Shape::Instance(type_name)) => Some(type_name),
			_ => None,
		})
	}

	/// `render ("x" as pdf)` as the parser groups it: the overloaded function, its argument and the result type
	fn as_result<'a>(&self, items: &'a [Node]) -> Option<(&'a Node, &'a Node, String)> {
		let [head, argument] = items else { return None };
		let Node::Key(value, Op::As, result) = argument.drop_meta() else { return None };
		let result = result.drop_meta().name();
		self.overloads.get(&head.name())?.contains(&result).then_some((head, value.as_ref(), result))
	}

	fn picks(&self, value: &Node, result: &str) -> bool {
		self.overloaded_call(value).is_some_and(|(head, _)| self.overloads[&head.name()].iter().any(|candidate| candidate == result))
	}

	/// `docx d = …`: the type word of a typed declaration
	fn typed_declaration(&self, items: &[Node]) -> Option<String> {
		let [type_word, declaration] = items else { return None };
		let Node::Symbol(type_word) = type_word.drop_meta() else { return None };
		let is_declaration = matches!(declaration.drop_meta(), Node::Key(target, Op::Assign | Op::Define, _) if matches!(target.drop_meta(), Node::Symbol(_)));
		(is_declaration && self.is_type(type_word)).then(|| type_word.clone())
	}

	/// The declared parameter types of the function a call names
	fn argument_types(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Vec<Option<String>>> {
		is_call(items, bracket, separator).then(|| self.parameter_types.get(&items.first()?.name()).cloned()).flatten()
	}

	/// The variant the expected type names, else the first-declared one with a got-it warning
	fn resolve(&self, call: &Node, head: &Node, arguments: &[Node], expected: Option<&str>) -> Node {
		let name = head.name();
		let results = &self.overloads[&name];
		let arguments: Vec<Node> = arguments.iter().map(|argument| self.rewrite(argument.clone(), None)).collect();
		if let Some(result) = expected.filter(|expected| results.iter().any(|result| result == expected)) {
			return self.call(head, result, arguments);
		}
		let written = call.serialize().trim().to_string();
		let question = format!("{name} has variants returning {}: which does `{written}` mean?", results.join(", "));
		let readings = results.iter().map(|result| reading(result, &format!("{written} as {result}"))).collect();
		match ask(&Ask::new(TOPIC, question, readings, Fallback::Warning).written(&written).at_node(head)) {
			Ok(chosen) => self.call(head, &results[chosen], arguments),
			Err(error) => error,
		}
	}

	fn call(&self, head: &Node, result: &str, arguments: Vec<Node>) -> Node {
		let variant = Node::Symbol(witness_name(&head.name(), result));
		Node::List([vec![variant], arguments].concat(), Bracket::Round, Separator::None)
	}
}

/// Overloading by parameter types: `combine(a:float, b:float) := a+b; combine(a:int, b:int) := a*b` defines the variants
/// `combine·float·float` and `combine·int·int`; a call takes the variant its arguments fit best (an exact match before a
/// widening Int → Float), and an argument of unknown type or a tie is a loud error naming the variants.
fn lower_parameter_overloads(node: Node) -> Node {
	let mut signatures: HashMap<String, Vec<Vec<Option<String>>>> = HashMap::new();
	collect_signatures(&node, &mut signatures);
	let overloaded: HashMap<String, Vec<Vec<String>>> = signatures
		.into_iter()
		.filter(|(_, variants)| variants.len() > 1)
		.filter_map(|(name, variants)| {
			let variants: Vec<Vec<String>> = variants.into_iter().map(|types| types.into_iter().collect::<Option<_>>()).collect::<Option<_>>()?;
			let distinct = variants.iter().enumerate().all(|(index, types)| !variants[..index].contains(types));
			// parameters of declared types dispatch through traits (traits.rs)
			let builtin = variants.iter().flatten().all(|type_name| type_word_kind(type_name).is_some());
			(distinct && builtin).then_some((name, variants))
		})
		.collect();
	if overloaded.is_empty() {
		return node;
	}
	let mut variables = HashMap::new();
	collect_variable_kinds(&node, &mut variables);
	let mut renaming = ParameterOverloads { overloaded, variables, seen: HashMap::new() };
	renaming.rewrite(node)
}

/// The argument counts a definition accepts: its parameters without a default are required
#[derive(Clone, Copy, PartialEq)]
struct Arity {
	required: usize,
	total: usize,
}

impl Arity {
	fn of(parameters: &[Node]) -> Self {
		let required = parameters.iter().filter(|parameter| !matches!(parameter.drop_meta(), Node::Key(_, Op::Assign, _))).count();
		Arity { required, total: parameters.len() }
	}

	fn accepts(&self, count: usize) -> bool {
		(self.required..=self.total).contains(&count)
	}
}

/// Definitions of one name with different parameter counts (C#, Kotlin, Julia overloading by arity) become separate
/// functions `f·1`, `f·2`; every call names the one its argument count fits, so dispatch stays static
pub fn lower_arity_overloads(node: Node) -> Node {
	let mut arities: HashMap<String, Vec<Arity>> = HashMap::new();
	each_definition(&node, &mut |definition| arities.entry(definition.head[0].name()).or_default().push(Arity::of(&definition.head[1..])));
	arities.retain(|_, variants| variants.len() > 1 && variants.iter().enumerate().all(|(index, arity)| variants[..index].iter().all(|other| other.total != arity.total)));
	if arities.is_empty() {
		return node;
	}
	ArityOverloads { arities }.rewrite(node)
}

/// Calls `visit` with every definition, also the ones nested in bodies
fn each_definition<'a>(node: &'a Node, visit: &mut impl FnMut(&Definition<'a>)) {
	if let Some(definition) = definition(node) {
		visit(&definition);
		return each_definition(definition.body, visit);
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => {
			each_definition(left, visit);
			each_definition(right, visit);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| each_definition(item, visit)),
		_ => {}
	}
}

struct ArityOverloads {
	arities: HashMap<String, Vec<Arity>>,
}

fn arity_variant(name: &str, arity: Arity) -> String {
	witness_name(name, &arity.total.to_string())
}

/// A pass that renames the definitions of overloaded names to their variants and every call to the variant it fits
trait VariantRenaming {
	/// The variant name of this definition of `name`, None when the name is not overloaded
	fn renamed(&mut self, name: &str, parameters: &[Node]) -> Option<String>;
	fn resolve_call(&mut self, node: &Node) -> Option<Node>;

	fn rewrite(&mut self, node: Node) -> Node {
		if let Some(renamed) = self.rewrite_definition(&node) {
			return renamed;
		}
		if let Some(call) = self.resolve_call(&node) {
			return call;
		}
		match node {
			Node::Key(left, op, right) => Node::Key(Box::new(self.rewrite(*left)), op, Box::new(self.rewrite(*right))),
			Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| self.rewrite(item)).collect(), bracket, separator),
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.rewrite(*node)), data },
			other => other,
		}
	}

	fn rewrite_definition(&mut self, node: &Node) -> Option<Node> {
		let definition = definition(node)?;
		let Node::Key(target, op, _) = node.drop_meta() else { return None };
		let body = Box::new(self.rewrite(definition.body.clone()));
		let Some(variant) = self.renamed(&definition.head[0].name(), &definition.head[1..]) else { return Some(Node::Key(target.clone(), *op, body)) };
		let mut head = definition.head.to_vec();
		head[0] = Node::Symbol(variant);
		let head = Node::List(head, Bracket::Round, Separator::None);
		let target = match target.drop_meta() {
			Node::Key(_, result_op @ (Op::Colon | Op::As), result) => Node::Key(Box::new(head), *result_op, result.clone()),
			_ => head,
		};
		Some(Node::Key(Box::new(target), *op, body))
	}
}

impl VariantRenaming for ArityOverloads {
	fn renamed(&mut self, name: &str, parameters: &[Node]) -> Option<String> {
		self.arities.contains_key(name).then(|| arity_variant(name, Arity::of(parameters)))
	}

	/// `f(1, 2)`, `f 1 2`: the call of the definition that accepts two arguments
	fn resolve_call(&mut self, node: &Node) -> Option<Node> {
		let Node::List(items, bracket, separator) = node.drop_meta() else { return None };
		let [head, arguments @ ..] = items.as_slice() else { return None };
		let name = head.drop_meta().name();
		let variants = self.arities.get(&name)?.clone();
		if !is_call(items, bracket, separator) {
			return None;
		}
		let fitting: Vec<&Arity> = variants.iter().filter(|arity| arity.accepts(arguments.len())).collect();
		let written = format!("{name}({})", arguments.iter().map(|argument| argument.serialize().trim().to_string()).collect::<Vec<_>>().join(", "));
		let chosen = match fitting.as_slice() {
			[arity] => **arity,
			[] => {
				let mut counts: Vec<usize> = variants.iter().flat_map(|arity| arity.required..=arity.total).collect();
				counts.sort();
				counts.dedup();
				let counts = counts.iter().map(usize::to_string).collect::<Vec<_>>().join(" or ");
				return Some(crate::diagnostic::Diagnostic::at(head, format!("{name} takes {counts} arguments, got {}", arguments.len())).into_error());
			}
			_ => return Some(crate::diagnostic::Diagnostic::at(head, format!("`{written}` fits two definitions of {name}: pass every argument of the one you mean")).into_error()),
		};
		let arguments: Vec<Node> = arguments.iter().map(|argument| self.rewrite(argument.clone())).collect();
		Some(Node::List([vec![Node::Symbol(arity_variant(&name, chosen))], arguments].concat(), Bracket::Round, Separator::None))
	}
}

fn collect_signatures(node: &Node, signatures: &mut HashMap<String, Vec<Vec<Option<String>>>>) {
	each_definition(node, &mut |definition| signatures.entry(definition.head[0].name()).or_default().push(definition.head[1..].iter().map(parameter_type).collect()));
}

/// The kind a variable is assigned everywhere in the program; a variable assigned values of different kinds has none
fn collect_variable_kinds(node: &Node, variables: &mut HashMap<String, Option<Kind>>) {
	node.visit(&mut |part| {
		let Node::Key(target, Op::Assign | Op::Define, value) = part else { return };
		let Node::Symbol(name) = target.drop_meta() else { return };
		let kind = argument_kind(value, &HashMap::new());
		variables.entry(name.clone()).and_modify(|known| if *known != kind { *known = None }).or_insert(kind);
	});
}

/// The kind of an argument as overloads see it: a decimal literal (exact `1.1` too) is a Float here, it fits `x:float`
/// exactly and never `x:int`
fn argument_kind(argument: &Node, variables: &HashMap<String, Option<Kind>>) -> Option<Kind> {
	use crate::extensions::numbers::Number;
	match argument.drop_meta() {
		Node::Number(Number::Float(_) | Number::Quotient(..)) => Some(Kind::Float),
		Node::Number(_) => Some(Kind::Int),
		Node::Text(_) => Some(Kind::Text),
		Node::Char(_) => Some(Kind::Codepoint),
		Node::Symbol(name) => variables.get(name).copied().flatten(),
		Node::Key(_, Op::As, type_node) => type_word_kind(&type_node.drop_meta().name()),
		Node::Key(left, op, right) if op.is_arithmetic() => match (argument_kind(left, variables)?, argument_kind(right, variables)?) {
			(Kind::Int, Kind::Int) => Some(Kind::Int),
			(Kind::Float | Kind::Int, Kind::Float | Kind::Int) => Some(Kind::Float),
			_ => None,
		},
		Node::List(items, Bracket::Round, _) if items.len() == 1 => argument_kind(&items[0], variables),
		_ => None,
	}
}

/// How well an argument of `kind` fits a parameter of `type_name`: 0 exactly, 1 by widening an Int to a Float or any
/// number to the wider `number`
fn fit(kind: Kind, type_name: &str) -> Option<u32> {
	if type_name == "number" {
		return matches!(kind, Kind::Int | Kind::Float).then_some(1);
	}
	match (kind, type_word_kind(type_name)?) {
		(given, wanted) if given == wanted => Some(0),
		(Kind::Int, Kind::Float) => Some(1),
		(Kind::Codepoint, Kind::Text) => Some(1),
		_ => None,
	}
}

struct ParameterOverloads {
	overloaded: HashMap<String, Vec<Vec<String>>>,
	variables: HashMap<String, Option<Kind>>,
	/// Definitions of each overloaded name renamed so far, in declaration order
	seen: HashMap<String, usize>,
}

impl VariantRenaming for ParameterOverloads {
	fn renamed(&mut self, name: &str, _parameters: &[Node]) -> Option<String> {
		let variants = self.overloaded.get(name)?;
		let index = self.seen.entry(name.to_string()).or_default();
		let variant = Self::variant_name(name, &variants[*index]);
		*index += 1;
		Some(variant)
	}

	fn resolve_call(&mut self, node: &Node) -> Option<Node> {
		self.resolve_typed_call(node)
	}
}

impl ParameterOverloads {
	fn variant_name(name: &str, types: &[String]) -> String {
		types.iter().fold(name.to_string(), |variant, type_name| witness_name(&variant, type_name))
	}

	/// `combine(1.1, 2.2)`: the call of the variant the arguments fit best
	fn resolve_typed_call(&mut self, node: &Node) -> Option<Node> {
		let Node::List(items, bracket, separator) = node.drop_meta() else { return None };
		let [head, arguments @ ..] = items.as_slice() else { return None };
		let variants = self.overloaded.get(&head.drop_meta().name())?.clone();
		if arguments.is_empty() || !is_call(items, bracket, separator) {
			return None;
		}
		let name = head.drop_meta().name();
		let kinds: Vec<Option<Kind>> = arguments.iter().map(|argument| argument_kind(argument, &self.variables)).collect();
		let scores: Vec<(u32, &Vec<String>)> = variants
			.iter()
			.filter(|types| types.len() == arguments.len())
			.filter_map(|types| {
				let fits: Option<Vec<u32>> = kinds.iter().zip(types.iter()).map(|(kind, type_name)| fit((*kind)?, type_name)).collect();
				Some((fits?.iter().sum(), types))
			})
			.collect();
		let best = scores.iter().map(|(score, _)| *score).min();
		let chosen: Vec<&Vec<String>> = scores.iter().filter(|(score, _)| Some(*score) == best).map(|(_, types)| *types).collect();
		let written = node.serialize().trim().to_string();
		let listed = variants.iter().map(|types| format!("{name}({})", types.join(", "))).collect::<Vec<_>>().join(", ");
		let arguments: Vec<Node> = arguments.iter().map(|argument| self.rewrite(argument.clone())).collect();
		match chosen.as_slice() {
			[types] => Some(Node::List([vec![Node::Symbol(Self::variant_name(&name, types))], arguments].concat(), Bracket::Round, Separator::None)),
			_ => Some(crate::diagnostic::Diagnostic::at(head, format!("`{written}` fits no single variant of {listed}: give the arguments a type, e.g. `x as float`")).into_error()),
		}
	}
}
