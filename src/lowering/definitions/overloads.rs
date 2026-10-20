//! D10 "Dispatch on return type" (notes/dispatch.md): definitions of one name with different result types are overloads,
//! renamed `render·pdf`, `render·docx` in declaration order. A call picks the overload its expected result type names
//! (`render "x" as pdf`, `docx d = render "x"`, `d:docx = …`, an argument of a parameter `d:docx`); without one the
//! first-declared overload is taken with a got-it warning naming the explicit form.
//! A function f with a companion f_as(…, type) gets its expected type that way: `x:int = agent "33/3"` is
//! `x:int = agent_as("33/3", "int")` (card int-let, lib/agent.warp).

use super::nodes::{
	assign, call, children_rewritten, in_block_as_written, is_assigned_data, is_call_head, is_words_call, key,
	with_parts_rewritten,
};
use crate::analyzer::{call_name, collect_all_types, type_word_kind};
use crate::errors::diagnostic::{ask, reading, Ask, Fallback};
use crate::lowering::{function_head, DefinitionForm};
use crate::node::{parens, Bracket, Node, Separator};
use crate::operators::Op;
use crate::traits::{witness_name, witness_operation, InstanceTypes, Shape};
use crate::type_kinds::{Kind, TypeRegistry};
use crate::words::ANY_TYPE;
use std::collections::{HashMap, HashSet};

const TOPIC: &str = "return-type";
/// `agent_as`: the companion of `agent` that takes the expected result type as its last argument
const COMPANION_SUFFIX: &str = "_as";
/// How well a value fits `any`: worse than every typed fit, so the untyped variant is the fallback
const ANY_FIT: u32 = 100;
/// How well a list of known elements fits a plain `list`: worse than a list of its elements, even widened
const LIST_FIT: u32 = 2;

/// The function f whose companion f_as is `name`
pub fn companion_of(name: &str) -> Option<&str> {
	name.strip_suffix(COMPANION_SUFFIX)
}

/// A definition `name(params) := body`, `name(params):T := body` or `name(params) as T := body`
struct Definition<'a> {
	head: &'a [Node],
	written_result: Option<String>,
	body: &'a Node,
}

fn definition(node: &Node) -> Option<Definition<'_>> {
	let definition = function_head(node)
		.filter(|definition| matches!(definition.form, DefinitionForm::Operator(_)) && is_call_head(definition.head))?;
	let written_result = match definition.result {
		Some(result) => Some(result.drop_meta().symbol_name()?.to_string()),
		None => None,
	};
	let Node::List(head, _, _) = definition.head.drop_meta() else {
		return None;
	};
	Some(Definition { head, written_result, body: definition.body })
}

pub fn lower(node: Node) -> Node {
	let node = lower_parameter_overloads(node);
	let mut registry = TypeRegistry::new();
	collect_all_types(&mut registry, &node);
	let mut lowering = Lowering {
		registry,
		types: InstanceTypes::of(&node),
		overloads: HashMap::new(),
		single_results: HashMap::new(),
		parameter_types: HashMap::new(),
		companions: HashSet::new(),
	};
	let mut results: Vec<(String, Option<String>)> = vec![];
	lowering.collect(&node, &mut results);
	lowering.overloads = overloads(&results);
	lowering.single_results = single_results(&results);
	// by operation: the arity variants agent·1 and agent·2 are agent (lower_arity_overloads ran before)
	let defined: HashSet<&str> = results.iter().map(|(name, _)| witness_operation(name)).collect();
	lowering.companions = defined
		.iter()
		.filter(|name| defined.contains(format!("{name}{COMPANION_SUFFIX}").as_str()))
		.map(|name| name.to_string())
		.collect();
	// definitions that differ in a parameter type (methods of two classes) dispatch on it (traits.rs), not on their result
	let mut signatures = HashMap::new();
	collect_signatures(&node, &mut signatures);
	lowering.overloads.retain(|name, _| {
		signatures.get(name).is_none_or(|variants| variants.iter().all(|types| *types == variants[0]))
	});
	if lowering.overloads.is_empty()
		&& lowering.companions.is_empty()
		&& !results.iter().any(|(_, result)| result.as_deref().is_some_and(|result| lowering.is_declared(result)))
	{
		return node;
	}
	lowering.rewrite(node, None)
}

/// The call of a function `named` with its arguments: `f "x"`, `f("x")`, `(f "x")`
fn named_call<'a>(node: &'a Node, named: &dyn Fn(&str) -> bool) -> Option<(&'a Node, &'a [Node])> {
	let Node::List(items, bracket, separator) = node.drop_meta() else {
		return None;
	};
	if let ([single], Bracket::Round) = (items.as_slice(), bracket) {
		return named_call(single, named);
	}
	let [head, arguments @ ..] = items.as_slice() else {
		return None;
	};
	(is_call(items, bracket, separator) && !arguments.is_empty() && named(&head.name())).then_some((head, arguments))
}

/// `f(x)`, `f x`, `(f x)`, `{f x}`
fn is_call(items: &[Node], bracket: &Bracket, separator: &Separator) -> bool {
	call_name(items, bracket, separator).is_some() || is_words_call(bracket, separator)
}

fn bracket_of(node: &Node) -> Bracket {
	match node.drop_meta() {
		Node::List(_, bracket, _) => bracket.clone(),
		_ => Bracket::None,
	}
}

pub(crate) fn parameter_type(parameter: &Node) -> Option<String> {
	match parameter.drop_meta() {
		// `b: [int]` is a `list of int`
		Node::Key(_, Op::Colon, type_node) => {
			let list_type = crate::lowering::declaration_forms::bracketed_list_type(type_node);
			Some(list_type.as_ref().unwrap_or(type_node.drop_meta()).name())
		}
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

/// Names defined once with a known result type: that type
fn single_results(results: &[(String, Option<String>)]) -> HashMap<String, String> {
	results
		.iter()
		.filter(|(name, _)| results.iter().filter(|(other, _)| other == name).count() == 1)
		.filter_map(|(name, result)| Some((name.clone(), result.clone()?)))
		.collect()
}

struct Lowering {
	registry: TypeRegistry,
	types: InstanceTypes,
	overloads: HashMap<String, Vec<String>>,
	/// functions defined once, with a known result type: `render "x" as pdf` names their result, no conversion
	single_results: HashMap<String, String>,
	parameter_types: HashMap<String, Vec<Option<String>>>,
	/// the functions f (by operation, witness_operation) with a companion f_as (COMPANION_SUFFIX)
	companions: HashSet<String>,
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
		named_call(node, &|name| self.overloads.contains_key(name))
	}

	/// A call of a function with a companion (`agent "33/3"`) and its arguments
	fn companion_call<'a>(&self, node: &'a Node) -> Option<(&'a Node, &'a [Node])> {
		named_call(node, &|name| {
			self.companions.contains(witness_operation(name)) && !self.overloads.contains_key(name)
		})
	}

	/// The call of the variant or the companion that gives `result`
	fn typed_call(&self, head: &Node, result: &str, arguments: Vec<Node>) -> Node {
		if self.overloads.contains_key(&head.name()) {
			return self.call(head, result, arguments);
		}
		if self.single_results.contains_key(&head.name()) {
			return parens([vec![head.clone()], arguments].concat());
		}
		let companion = call(
			&format!("{}{COMPANION_SUFFIX}", witness_operation(&head.name())),
			[arguments, vec![Node::Text(result.to_string())]].concat(),
		);
		// the companion's answer has no static type: `as bool` keeps the type it was asked for, so `yes` shows as yes, not 1
		match type_word_kind(result) {
			Some(_) => key(companion, Op::As, Node::Symbol(result.to_string())),
			None => companion,
		}
	}

	fn rewrite(&self, node: Node, expected: Option<&str>) -> Node {
		if let Some(rewritten) = self.rewrite_definition(&node) {
			return rewritten;
		}
		if let Some((head, arguments)) = self.overloaded_call(&node) {
			return in_block_as_written(&bracket_of(&node), self.resolve(&node, head, arguments, expected));
		}
		if let (Some(expected), Some((head, arguments))) = (expected, self.companion_call(&node)) {
			let arguments = arguments.iter().map(|argument| self.rewrite(argument.clone(), None)).collect();
			return in_block_as_written(&bracket_of(&node), self.typed_call(head, expected, arguments));
		}
		match node {
			// `(render "x") as pdf`
			Node::Key(value, Op::As, result) if self.picks(&value, &result.drop_meta().name()) => {
				self.rewrite(*value, Some(&result.drop_meta().name()))
			}
			Node::Key(target, Op::Assign, value) if is_assigned_data(&target, &value) => {
				assign(*target, with_parts_rewritten(*value, &mut |part| self.rewrite(part, None)))
			}
			// `d:docx = render "x"`
			Node::Key(target, op @ (Op::Assign | Op::Define), value) => {
				let declared = match target.drop_meta() {
					Node::Key(_, Op::Colon, type_node) => Some(type_node.drop_meta().name()),
					_ => None,
				};
				Node::Key(target, op, Box::new(self.rewrite(*value, declared.as_deref())))
			}
			Node::Key(left, op, right) => key(self.rewrite(*left, None), op, self.rewrite(*right, None)),
			// `docx d = render "x"`; of a declared type it is the assignment, the value has the type
			Node::List(items, Bracket::None, Separator::Space) if self.typed_declaration(&items).is_some() => {
				let declared = self.typed_declaration(&items).expect("checked");
				let mut items = items.into_iter();
				let type_word = items.next().expect("checked");
				let Some(Node::Key(target, op, value)) =
					items.next().map(|declaration| declaration.drop_meta().clone())
				else {
					unreachable!("checked")
				};
				let declaration = Node::Key(target, op, Box::new(self.rewrite(*value, Some(&declared))));
				if self.is_declared(&declared) {
					declaration
				} else {
					Node::List(vec![type_word, declaration], Bracket::None, Separator::Space)
				}
			}
			// `(render "x")` passes the expected type on
			Node::List(items, Bracket::Round, separator) if items.len() == 1 => {
				let item = items.into_iter().next().expect("one item");
				Node::List(vec![self.rewrite(item, expected)], Bracket::Round, separator)
			}
			Node::List(items, bracket, separator) => {
				let expected_arguments = self.argument_types(&items, &bracket, &separator);
				let items = items.into_iter().enumerate().map(|(index, item)| {
					let expected = expected_arguments
						.as_ref()
						.and_then(|types| types.get(index.wrapping_sub(1)).cloned().flatten());
					self.rewrite(item, expected.as_deref())
				});
				Node::List(items.collect(), bracket, separator)
			}
			other => children_rewritten(other, |child| self.rewrite(child, expected)),
		}
	}

	/// A definition: an overload renamed to its variant, a result type naming a declared type dropped from the head (the
	/// body constructs it); its body rewritten
	fn rewrite_definition(&self, node: &Node) -> Option<Node> {
		let definition = definition(node)?;
		let Node::Key(target, op, _) = node.drop_meta() else {
			return None;
		};
		let name = definition.head[0].name();
		let body = Box::new(self.rewrite(definition.body.clone(), None));
		let variant = self
			.overloads
			.contains_key(&name)
			.then(|| self.result_type(&definition))
			.flatten()
			.map(|result| witness_name(&name, &result));
		let drops_result = definition.written_result.as_deref().is_some_and(|result| self.is_declared(result));
		if variant.is_none() && !drops_result {
			return Some(Node::Key(target.clone(), *op, body));
		}
		let mut head = definition.head.to_vec();
		if let Some(variant) = variant {
			head[0] = Node::Symbol(variant);
		}
		Some(Node::Key(Box::new(parens(head)), *op, body))
	}

	/// The written result type, else the type of the instance the body constructs
	fn result_type(&self, definition: &Definition) -> Option<String> {
		definition.written_result.clone().or_else(|| match self.types.shape(definition.body) {
			Some(Shape::Instance(type_name)) => Some(type_name),
			_ => None,
		})
	}

	/// Whether the function `name` can give a `result`: an overload of it, its companion or its only definition does
	fn gives(&self, name: &str, result: &str) -> bool {
		match self.overloads.get(name) {
			Some(results) => results.iter().any(|candidate| candidate == result),
			None => {
				self.companions.contains(witness_operation(name))
					|| self.single_results.get(name).is_some_and(|single| single == result)
			}
		}
	}

	/// `(render "x") as pdf`: the call already gives the type
	fn picks(&self, value: &Node, result: &str) -> bool {
		named_call(value, &|name| self.gives(name, result)).is_some()
	}

	/// `docx d = …`: the type word of a typed declaration
	fn typed_declaration(&self, items: &[Node]) -> Option<String> {
		let [type_word, declaration] = items else { return None };
		let type_word = type_word.symbol_name()?;
		let is_declaration = matches!(declaration.drop_meta(), Node::Key(target, Op::Assign | Op::Define, _) if matches!(target.drop_meta(), Node::Symbol(_)));
		(is_declaration && self.is_type(type_word)).then(|| type_word.to_string())
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
		// `(render "x") as pdf`: grouped, as `render "x" as pdf` casts the argument (card parameter-named-number)
		let readings = results.iter().map(|result| reading(result, &format!("({written}) as {result}"))).collect();
		match ask(&Ask::new(TOPIC, question, readings, Fallback::Warning).written(&written).at_node(head)) {
			Ok(chosen) => self.call(head, &results[chosen], arguments),
			Err(error) => error,
		}
	}

	fn call(&self, head: &Node, result: &str, arguments: Vec<Node>) -> Node {
		let variant = Node::Symbol(witness_name(&head.name(), result));
		parens([vec![variant], arguments].concat())
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
			let variants: Vec<Vec<String>> = variants
				.into_iter()
				.map(|types| types.into_iter().map(|type_name| type_name.unwrap_or(ANY_TYPE.to_string())).collect())
				.collect();
			let typed = variants.iter().flatten().any(|type_name| type_name != ANY_TYPE);
			let distinct = variants.iter().enumerate().all(|(index, types)| !variants[..index].contains(types));
			// parameters of declared types dispatch through traits (traits.rs)
			let builtin = variants.iter().flatten().all(|type_name| is_overload_type(type_name));
			(distinct && builtin && typed).then_some((name, variants))
		})
		.collect();
	if overloaded.is_empty() {
		return node;
	}
	let variables =
		crate::analyzer::agreeing_kinds(&node, HashMap::new(), |_, value| Some(argument_kind(value, &HashMap::new())));
	let mut registry = TypeRegistry::new();
	collect_all_types(&mut registry, &node);
	let classes: HashSet<String> = registry.types().iter().map(|type_def| type_def.name.clone()).collect();
	let list_elements = list_element_variables(&node, &classes, &variables);
	let mut renaming = ParameterOverloads {
		overloaded,
		variables,
		classes,
		list_elements,
		seen: HashMap::new(),
		declared: vec![HashMap::new()],
	};
	renaming.rewrite(node)
}

/// Variables assigned lists of one element type, `notes = [note{…}, note{…}]`: that type, None where the assignments
/// disagree or the items do
fn list_element_variables(
	node: &Node,
	classes: &HashSet<String>,
	variables: &HashMap<String, Option<Kind>>,
) -> HashMap<String, Option<String>> {
	let mut elements: HashMap<String, Option<String>> = HashMap::new();
	node.visit(&mut |node| {
		let Node::Key(target, Op::Assign | Op::Define, value) = node else {
			return;
		};
		let Node::Symbol(name) = target.drop_meta() else {
			return;
		};
		let element = literal_element_type(value, classes, variables);
		elements
			.entry(name.clone())
			.and_modify(|known| {
				if *known != element {
					*known = None
				}
			})
			.or_insert(element);
	});
	elements
}

/// The element type of a list written with its items: `int` of `[1, 2]`, `note` of `[note{…}]`; None for an empty or
/// mixed list
fn literal_element_type(
	value: &Node,
	classes: &HashSet<String>,
	variables: &HashMap<String, Option<Kind>>,
) -> Option<String> {
	let Node::List(items, Bracket::Square, _) = value.ungrouped() else {
		return None;
	};
	let mut types = items.iter().map(|item| item_type(item, classes, variables));
	let first = types.next()??;
	types.all(|item_type| item_type.as_ref() == Some(&first)).then_some(first)
}

/// `note` of an instance `note{…}` of a declared type, `int` of `1`
fn item_type(item: &Node, classes: &HashSet<String>, variables: &HashMap<String, Option<Kind>>) -> Option<String> {
	match item.ungrouped() {
		Node::Key(class, Op::Colon, fields) if matches!(fields.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			class.symbol_name().filter(|name| classes.contains(*name)).map(str::to_string)
		}
		other => argument_kind(other, variables).map(|kind| kind.to_string()),
	}
}

/// The argument counts a definition accepts: its parameters without a default are required
#[derive(Clone, Copy, PartialEq)]
struct Arity {
	required: usize,
	total: usize,
}

impl Arity {
	fn of(parameters: &[Node]) -> Self {
		let required =
			parameters.iter().filter(|parameter| !matches!(parameter.drop_meta(), Node::Key(_, Op::Assign, _))).count();
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
	// definitions of one arity (`f(x)`, `f(b: Box)`) are one variant, which dispatches on the type later
	each_definition(&node, &mut |definition| {
		let variants = arities.entry(definition.head[0].name()).or_default();
		let arity = Arity::of(&definition.head[1..]);
		if !variants.contains(&arity) {
			variants.push(arity);
		}
	});
	arities.retain(|_, variants| {
		variants.len() > 1
			&& variants
				.iter()
				.enumerate()
				.all(|(index, arity)| variants[..index].iter().all(|other| other.total != arity.total))
	});
	if arities.is_empty() {
		return node;
	}
	let called_parameters = called_parameters(&node);
	let class_fields = crate::lowering::class_methods::class_fields(&node);
	let classes: Vec<String> = class_fields.keys().cloned().collect();
	let node = MethodVariants {
		arities: &arities,
		class_fields: class_fields
			.into_iter()
			.map(|(class, fields)| (class, fields.into_iter().map(|(field, _)| field).collect()))
			.collect(),
		receiver_classes: receiver_classes(&node),
		instances: crate::lowering::class_methods::instance_classes(&node, &classes),
		maps: map_variables(&node),
	}
	.rewrite(node);
	let node = ArityOverloads { arities: arities.clone() }.rewrite(node);
	passed_variants(node, &arities, &called_parameters)
}

/// `p.label`, `p.label(x)` and `p.label = v` of label overloaded by arity: the variant taking the receiver too, label·1,
/// label·2 (read `label·2(p, x)`, set `p = label·2(p, v)` by library_words). The receiver decides (decision
/// default-methods): its own member, a map's key or a class's field, wins; a variant whose first parameter is a class
/// (`label(it: Point, v)`) is only for instances of that class
struct MethodVariants<'a> {
	arities: &'a HashMap<String, Vec<Arity>>,
	/// class → its fields
	class_fields: HashMap<String, Vec<String>>,
	/// (name, parameter count) → the class its first parameter is typed with: `label(it: Point, v)`
	receiver_classes: HashMap<(String, usize), String>,
	/// variable → the class of the instance it holds
	instances: HashMap<String, String>,
	/// variable → the keys of the map literal it holds: `m = {pick: 1}`
	maps: HashMap<String, Vec<String>>,
}

impl MethodVariants<'_> {
	/// The class of the instance `receiver` is: a variable holding one or a construction `Point(1, 2)`
	fn receiver_class(&self, receiver: &Node) -> Option<String> {
		match receiver.drop_meta() {
			Node::Symbol(name) => self.instances.get(name).cloned(),
			Node::List(items, Bracket::Round, _) => {
				items.first().map(|head| head.drop_meta().name()).filter(|class| self.class_fields.contains_key(class))
			}
			_ => None,
		}
	}

	/// `m.pick` of `m = {pick: 1}`, `p.name` of a Point with a field name: the receiver's own member
	fn is_own_member(&self, receiver: &Node, name: &str) -> bool {
		let map_keys = receiver.drop_meta().symbol_name().and_then(|variable| self.maps.get(variable));
		let fields = self.receiver_class(receiver).and_then(|class| self.class_fields.get(&class));
		map_keys.into_iter().chain(fields).any(|members| members.iter().any(|member| member == name))
	}

	/// Is the receiver's type known: a class instance, a map variable or a literal (`3.f`); an unknown one (a call's
	/// result, a parameter) keeps the member lookup at run time
	fn is_known_receiver(&self, receiver: &Node) -> bool {
		let literal = matches!(
			receiver.drop_meta(),
			Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::List(_, Bracket::Square, _)
		);
		let map = receiver.drop_meta().symbol_name().is_some_and(|variable| self.maps.contains_key(variable));
		literal || map || self.receiver_class(receiver).is_some()
	}

	/// The variant of `member` called on `receiver` with `arguments` in all, the receiver counted
	fn renamed(&self, receiver: &Node, member: &Node, arguments: usize) -> Option<Node> {
		let name = member.drop_meta().symbol_name()?;
		if !self.is_known_receiver(receiver) || self.is_own_member(receiver, name) {
			return None;
		}
		let fitting: Vec<&Arity> = self.arities.get(name)?.iter().filter(|arity| arity.accepts(arguments)).collect();
		let [arity] = fitting.as_slice() else { return None };
		match self.receiver_classes.get(&(name.to_string(), arity.total)) {
			Some(class) if self.receiver_class(receiver).as_ref() != Some(class) => None,
			_ => Some(Node::Symbol(arity_variant(name, **arity))),
		}
	}

	/// `p.label = v` calls the setter only for a variant typed for a class (`Point.label(v)`, `label(it: Point, v)`) on
	/// an instance of it; an untyped variant never turns an assignment into a call
	fn setter(&self, receiver: &Node, member: &Node) -> Option<Node> {
		let name = member.drop_meta().symbol_name()?;
		let class = self.receiver_classes.get(&(name.to_string(), 2))?;
		(self.receiver_class(receiver).as_ref() == Some(class)).then(|| self.renamed(receiver, member, 2)).flatten()
	}

	fn rewrite(&self, node: Node) -> Node {
		let recurse = |child: Node| self.rewrite(child);
		match node {
			Node::Key(target, Op::Assign, value) => match *target {
				// `p.label = v`: the setter, or the member itself
				Node::Key(receiver, Op::Dot, member) => {
					let member = self.setter(&receiver, &member).unwrap_or(*member);
					key(key(recurse(*receiver), Op::Dot, member), Op::Assign, recurse(*value))
				}
				target => key(recurse(target), Op::Assign, recurse(*value)),
			},
			Node::Key(receiver, Op::Dot, member) => {
				let member = match member.drop_meta().clone() {
					Node::List(items, Bracket::Round, separator) if !items.is_empty() => {
						let arguments = items.len();
						let mut items: Vec<Node> = items.into_iter().map(recurse).collect();
						if let Some(head) = self.renamed(&receiver, &items[0], arguments) {
							items[0] = head;
						}
						Node::List(items, Bracket::Round, separator)
					}
					symbol => self.renamed(&receiver, &symbol, 1).unwrap_or(*member),
				};
				key(recurse(*receiver), Op::Dot, member)
			}
			other => children_rewritten(other, recurse),
		}
	}
}

/// The keys of a map literal `{pick: 1}`
fn map_literal_keys(value: &Node) -> Option<Vec<String>> {
	let Node::List(items, Bracket::Curly, _) = value.drop_meta() else { return None };
	let keys = items.iter().map(|item| match item.drop_meta() {
		Node::Key(name, Op::Colon, _) => name.drop_meta().symbol_name().map(str::to_string),
		_ => None,
	});
	keys.collect()
}

/// variable → the keys of the map literal assigned to it
fn map_variables(node: &Node) -> HashMap<String, Vec<String>> {
	let mut maps = HashMap::new();
	node.visit(&mut |part| {
		if let Node::Key(variable, Op::Assign, value) = part {
			if let (Some(variable), Some(keys)) = (variable.drop_meta().symbol_name(), map_literal_keys(value)) {
				maps.insert(variable.to_string(), keys);
			}
		}
	});
	maps
}

/// (name, parameter count) → the class the first parameter of that definition is typed with: `f(it: P, v) := …`
fn receiver_classes(node: &Node) -> HashMap<(String, usize), String> {
	let mut classes = HashMap::new();
	each_definition(node, &mut |definition| {
		if let [name, receiver, ..] = definition.head {
			if let Node::Key(_, Op::Colon, class) = receiver.drop_meta() {
				classes.insert((name.name(), definition.head.len() - 1), class.drop_meta().name());
			}
		}
	});
	classes
}

/// (function, parameter index) → how many arguments the function's body calls that parameter with: 0 of
/// `twice(h) := h() + h()`, 1 of a parameter typed `f: (float) -> float`; a parameter passed on to such a one takes its
/// count, `play(tone) := play_signal(tone)` (card program-function), until no more do
fn called_parameters(node: &Node) -> HashMap<(String, usize), usize> {
	let mut called = HashMap::new();
	each_definition(node, &mut |definition| {
		for (index, parameter) in definition.head[1..].iter().enumerate() {
			let declared = crate::lowering::function_types::declared_arity(parameter);
			let Some(name) = super::nodes::parameter_name(parameter) else { continue };
			if let Some(arguments) = declared.or_else(|| called_arity(definition.body, &name)) {
				called.insert((definition.head[0].name(), index), arguments);
			}
		}
	});
	loop {
		let known = called.len();
		each_definition(node, &mut |definition| {
			for (index, parameter) in definition.head[1..].iter().enumerate() {
				let function = definition.head[0].name();
				let Some(name) = super::nodes::parameter_name(parameter) else { continue };
				if called.contains_key(&(function.clone(), index)) {
					continue;
				}
				if let Some(arguments) = passed_on_arity(definition.body, &name, &called) {
					called.insert((function, index), arguments);
				}
			}
		});
		if called.len() == known {
			return called;
		}
	}
}

/// How many arguments the function `parameter` is passed to calls it with: 1 of `apply(f, x)` when apply calls f(x)
fn passed_on_arity(body: &Node, parameter: &str, called: &HashMap<(String, usize), usize>) -> Option<usize> {
	let mut found = None;
	body.visit(&mut |node| {
		let Node::List(items, Bracket::Round, _) = node else { return };
		let Some((callee, arguments)) = items.split_first() else { return };
		for (index, argument) in arguments.iter().enumerate() {
			if found.is_none() && argument.drop_meta().is_symbol(parameter) {
				found = called.get(&(callee.drop_meta().name(), index)).copied();
			}
		}
	});
	found
}

/// How many arguments `body` calls `parameter` with: 1 of `h(x)`; none when its calls differ (`h(x)` and `h(x, y)`), so
/// no variant is picked silently for one of them
fn called_arity(body: &Node, parameter: &str) -> Option<usize> {
	let mut arities = Vec::new();
	body.visit(&mut |node| {
		if let Node::List(items, Bracket::Round, Separator::None) = node {
			if items.first().is_some_and(|head| head.drop_meta().is_symbol(parameter)) {
				arities.push(items.len() - 1);
			}
		}
	});
	arities.dedup();
	match arities.as_slice() {
		[arity] => Some(*arity),
		_ => None,
	}
}

/// `twice(f)`, `apply(f, 3)` of f overloaded by arity (card arity-overloaded): f is no one function value, so the
/// argument is the variant taking as many arguments as the called function calls that parameter with
fn passed_variants(
	node: Node,
	arities: &HashMap<String, Vec<Arity>>,
	called_parameters: &HashMap<(String, usize), usize>,
) -> Node {
	let node = children_rewritten(node, |child| passed_variants(child, arities, called_parameters));
	let Node::List(items, Bracket::Round, separator) = node else {
		return node;
	};
	let callee = items.first().map(|head| head.drop_meta().name()).unwrap_or_default();
	let variant = |index: usize, argument: &Node| {
		let base = argument.drop_meta().symbol_name()?;
		let arguments = *called_parameters.get(&(callee.clone(), index))?;
		let fitting = arities.get(base)?.iter().find(|arity| arity.accepts(arguments))?;
		Some(Node::Symbol(arity_variant(base, *fitting)))
	};
	let mut items = items.into_iter();
	let head = items.next();
	let arguments = items.enumerate().map(|(index, argument)| variant(index, &argument).unwrap_or(argument));
	Node::List(head.into_iter().chain(arguments).collect(), Bracket::Round, separator)
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

/// `f·1` of f given as a function called with one argument (card arity-overloaded): the arity overload taking that
/// many, when f is overloaded by arity (`f() := 0`, `f(x) := x * 2`) and `defined` knows it
pub(crate) fn variant_called_with(name: &str, arguments: usize, defined: impl Fn(&str) -> bool) -> Option<String> {
	let variant = witness_name(name, &arguments.to_string());
	(!defined(name) && defined(&variant)).then_some(variant)
}

/// `f·2`: the variant of f for two arguments that lower_arity_overloads made, which may still overload by type
pub(crate) fn is_arity_variant(name: &str) -> bool {
	name.split_once(crate::traits::WITNESS_SEPARATOR)
		.is_some_and(|(_, count)| !count.is_empty() && count.chars().all(|ch| ch.is_ascii_digit()))
}

/// A pass that renames the definitions of overloaded names to their variants and every call to the variant it fits
trait VariantRenaming {
	/// The variant name of this definition of `name`, None when the name is not overloaded
	fn renamed(&mut self, name: &str, parameters: &[Node]) -> Option<String>;
	fn resolve_call(&mut self, node: &Node) -> Option<Node>;
	/// A definition's body is rewritten between these two, its parameters in scope
	fn entered(&mut self, _parameters: &[Node]) {}
	fn left(&mut self) {}

	fn rewrite(&mut self, node: Node) -> Node {
		if let Some(renamed) = self.rewrite_definition(&node) {
			return renamed;
		}
		if let Some(call) = self.resolve_call(&node) {
			return in_block_as_written(&bracket_of(&node), call);
		}
		match node {
			Node::Key(left, Op::Assign, right) if is_assigned_data(&left, &right) => {
				assign(*left, with_parts_rewritten(*right, &mut |part| self.rewrite(part)))
			}
			other => children_rewritten(other, |child| self.rewrite(child)),
		}
	}

	fn rewrite_definition(&mut self, node: &Node) -> Option<Node> {
		let definition = definition(node)?;
		let Node::Key(target, op, _) = node.drop_meta() else {
			return None;
		};
		self.entered(&definition.head[1..]);
		let body = Box::new(self.rewrite(definition.body.clone()));
		self.left();
		let Some(variant) = self.renamed(&definition.head[0].name(), &definition.head[1..]) else {
			return Some(Node::Key(target.clone(), *op, body));
		};
		let mut head = definition.head.to_vec();
		head[0] = Node::Symbol(variant);
		let head = parens(head);
		let target = match target.drop_meta() {
			Node::Key(_, result_op @ (Op::Colon | Op::As), result) => {
				Node::Key(Box::new(head), *result_op, result.clone())
			}
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
		let Node::List(items, bracket, separator) = node.drop_meta() else {
			return None;
		};
		// `(quantity("5 m"))`: parentheses around a call are no call of it without arguments
		let [head, arguments @ ..] = items.as_slice() else {
			return None;
		};
		let name = head.symbol_name()?;
		let variants = self.arities.get(name)?.clone();
		if !is_call(items, bracket, separator) {
			return None;
		}
		let fitting: Vec<&Arity> = variants.iter().filter(|arity| arity.accepts(arguments.len())).collect();
		let written = format!(
			"{name}({})",
			arguments.iter().map(|argument| argument.serialize().trim().to_string()).collect::<Vec<_>>().join(", ")
		);
		let chosen = match fitting.as_slice() {
			[arity] => **arity,
			[] => {
				let mut counts: Vec<usize> = variants.iter().flat_map(|arity| arity.required..=arity.total).collect();
				counts.sort();
				counts.dedup();
				let counts = counts.iter().map(usize::to_string).collect::<Vec<_>>().join(" or ");
				return Some(
					crate::errors::diagnostic::Diagnostic::at(
						head,
						format!("{name} takes {counts} arguments, got {}", arguments.len()),
					)
					.into_error(),
				);
			}
			_ => {
				return Some(
					crate::errors::diagnostic::Diagnostic::at(
						head,
						format!("`{written}` fits two definitions of {name}: pass every argument of the one you mean"),
					)
					.into_error(),
				)
			}
		};
		let arguments: Vec<Node> = arguments.iter().map(|argument| self.rewrite(argument.clone())).collect();
		Some(call(&arity_variant(name, chosen), arguments))
	}
}

fn collect_signatures(node: &Node, signatures: &mut HashMap<String, Vec<Vec<Option<String>>>>) {
	each_definition(node, &mut |definition| {
		signatures
			.entry(definition.head[0].name())
			.or_default()
			.push(definition.head[1..].iter().map(parameter_type).collect())
	});
}

/// The kind of an argument as overloads see it: a decimal literal (exact `1.1` too) is a Float here, it fits `x:float`
/// exactly and never `x:int`
pub(crate) fn argument_kind(argument: &Node, variables: &HashMap<String, Option<Kind>>) -> Option<Kind> {
	use crate::extensions::numbers::Number;
	match argument.ungrouped() {
		Node::Number(Number::Float(_) | Number::Quotient(..)) => Some(Kind::Float),
		Node::Number(_) => Some(Kind::Int),
		Node::Text(_) => Some(Kind::Text),
		Node::Char(_) => Some(Kind::Codepoint),
		Node::Symbol(name) => variables.get(name).copied().flatten(),
		Node::Key(_, Op::As, type_node) => type_word_kind(&type_node.drop_meta().name()),
		Node::Key(left, op, right) if op.is_arithmetic() => {
			match (argument_kind(left, variables)?, argument_kind(right, variables)?) {
				(Kind::Int, Kind::Int) => Some(Kind::Int),
				(Kind::Float | Kind::Int, Kind::Float | Kind::Int) => Some(Kind::Float),
				_ => None,
			}
		}
		Node::List(_, Bracket::Square, _) => Some(Kind::List),
		_ => None,
	}
}

/// The kind a parameter type selects a variant by at compile time: a built-in type word, or a list type (`list`,
/// `ints`, `list of int`, `[int]`), held as no one kind elsewhere as it holds the empty list too
fn overload_kind(type_name: &str) -> Option<Kind> {
	match crate::analyzer::names_list_type(type_name) {
		true => Some(Kind::List),
		false => type_word_kind(type_name),
	}
}

/// A parameter type that takes part in an overload: `any`, a built-in type, a list type or a map, which no argument
/// kind known at compile time fits, so a map variant is picked at run time
fn is_overload_type(type_name: &str) -> bool {
	type_name == ANY_TYPE || overload_kind(type_name).is_some() || crate::analyzer::is_map_type(type_name)
}

/// How well an argument of `kind` fits a parameter of `type_name`: 0 exactly, 1 by widening an Int to a Float or any
/// number to the wider `number`
fn fit(kind: Kind, type_name: &str) -> Option<u32> {
	if type_name == ANY_TYPE {
		return Some(ANY_FIT);
	}
	if type_name == "number" {
		return matches!(kind, Kind::Int | Kind::Float).then_some(1);
	}
	match (kind, overload_kind(type_name)?) {
		(given, wanted) if given == wanted => Some(0),
		(Kind::Int, Kind::Float) => Some(1),
		(Kind::Codepoint, Kind::Text) => Some(1),
		_ => None,
	}
}

/// How well a list of `element`s fits a parameter of the list type `type_name`: its element as fit() takes a value,
/// a plain `list` worse than every list of a fitting element; an element only the run time knows fits every list
fn element_fit(element: Option<&str>, type_name: &str) -> Option<u32> {
	let Some(wanted) = crate::analyzer::list_element_type(type_name) else {
		return match type_name {
			ANY_TYPE => Some(ANY_FIT),
			_ if overload_kind(type_name) == Some(Kind::List) => Some(if element.is_some() { LIST_FIT } else { 0 }),
			_ => None,
		};
	};
	let Some(element) = element else { return Some(0) };
	match type_word_kind(element) {
		_ if crate::type_tests::canonical_spec_word(element) == crate::type_tests::canonical_spec_word(wanted) => {
			Some(0)
		}
		Some(kind) => fit(kind, wanted),
		None => None,
	}
}

/// `[file]` of `list of file`, as a cast writes it
fn bracketed(list_type: &str) -> String {
	match crate::analyzer::list_element_type(list_type) {
		Some(element) => format!("[{element}]"),
		None => list_type.to_string(),
	}
}

/// `[]` as written
fn is_empty_list(argument: &Node) -> bool {
	matches!(argument.ungrouped(), Node::List(items, Bracket::Square, _) if items.is_empty())
}

/// The run-time test of an argument for a parameter type; of a list type also of the list's first item, as a list keeps
/// no item type at run time (decision list-variant-dispatch: an empty list fits every list type)
fn run_time_test(argument: Node, type_name: &str) -> Option<Node> {
	let tested = |type_name: &str| {
		if takes_widened(type_name) {
			crate::words::NUMBER_WORD.to_string()
		} else {
			type_name.to_string()
		}
	};
	let test = crate::lowering::type_tests::declared_type_test(argument.clone(), &tested(type_name))?;
	let Some(element) = crate::analyzer::list_element_type(type_name) else {
		return Some(test);
	};
	let empty = key(call(crate::words::COUNT_WORD, vec![argument.clone()]), Op::Eq, Node::int(0));
	let first_item = key(argument, Op::Hash, Node::int(1));
	let item_test = crate::lowering::type_tests::declared_type_test(first_item, &tested(element))?;
	Some(key(test, Op::And, key(empty, Op::Or, item_test)))
}

/// `float` and `number` take any number, as fit() widens an Int: at run time an exact `1.5` is an Int node too
fn takes_widened(type_name: &str) -> bool {
	type_name == crate::words::FLOAT_WORD || type_name == crate::words::NUMBER_WORD
}

struct ParameterOverloads {
	overloaded: HashMap<String, Vec<Vec<String>>>,
	variables: HashMap<String, Option<Kind>>,
	/// The declared types, whose instances a list may hold
	classes: HashSet<String>,
	/// The element type of each variable assigned lists of one (list_element_variables)
	list_elements: HashMap<String, Option<String>>,
	/// Definitions of each overloaded name renamed so far, in declaration order
	seen: HashMap<String, usize>,
	/// The parameters and declared variables in scope with their declared type (`n: any`, `leaf: any = "abc"`; none
	/// for an untyped parameter, which hides an outer one), innermost last
	declared: Vec<HashMap<String, Option<String>>>,
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
		if let Node::Key(declaration, Op::Assign, _) = node.drop_meta() {
			self.declare(declaration, false);
		}
		self.resolve_typed_call(node)
	}

	fn entered(&mut self, parameters: &[Node]) {
		self.declared.push(HashMap::new());
		parameters.iter().for_each(|parameter| self.declare(parameter, true));
	}

	fn left(&mut self) {
		self.declared.pop();
	}
}

impl ParameterOverloads {
	/// `n: any`, `leaf: any = …`: a parameter or a declared variable of the innermost scope, with its declared type
	fn declare(&mut self, declaration: &Node, is_parameter: bool) {
		let type_name = parameter_type(declaration);
		let Some(name) = super::nodes::parameter_name(declaration).filter(|_| is_parameter || type_name.is_some())
		else {
			return;
		};
		self.declared.last_mut().expect("the program's scope").insert(name, type_name);
	}

	/// The type an argument naming a declared variable was declared with, `any` only where a variant takes `any` there
	/// (else the run time tests the value)
	fn declared_type(&self, argument: &Node, position: usize, variants: &[Vec<String>]) -> Option<String> {
		let name = argument.ungrouped().symbol_name()?;
		let type_name = self.declared.iter().rev().find_map(|scope| scope.get(name))?.as_ref()?;
		let any_variant = variants.iter().any(|types| types[position] == ANY_TYPE);
		(type_name != ANY_TYPE || any_variant).then(|| type_name.clone())
	}

	/// The element type of a list argument as far as the compile knows it: of its declared list type, of its items
	/// written, of the lists assigned to the variable it names
	fn element_type(&self, argument: &Node, declared: Option<&str>) -> Option<String> {
		if let Some(element) = declared.and_then(crate::analyzer::list_element_type) {
			return Some(element.to_string());
		}
		match argument.ungrouped() {
			Node::Symbol(name) => self.list_elements.get(name).cloned().flatten(),
			list => literal_element_type(list, &self.classes, &self.variables),
		}
	}

	/// `[] as [file]`, `[] as files`, `[] as ints`: the list and the list type it is cast to, which picks the variant
	fn as_list_type(&self, argument: &Node) -> Option<(Node, String)> {
		let Node::Key(list, Op::As, type_node) = argument.ungrouped() else {
			return None;
		};
		let type_name = match type_node.drop_meta() {
			Node::List(items, Bracket::Square, _) => match items.as_slice() {
				[element] => format!("{}{}", crate::analyzer::LIST_OF_PREFIX, element.symbol_name()?),
				_ => return None,
			},
			Node::Symbol(word) if crate::analyzer::names_list_type(word) => word.clone(),
			Node::Symbol(word) => {
				let classes: Vec<&str> = self.classes.iter().map(String::as_str).collect();
				let class = crate::extensions::strings::named_or_plural(&classes, word)
					.filter(|class| *class != word.as_str())?;
				format!("{}{class}", crate::analyzer::LIST_OF_PREFIX)
			}
			_ => return None,
		};
		Some((list.as_ref().clone(), type_name))
	}

	fn variant_name(name: &str, types: &[String]) -> String {
		// `f·list_of_int`: one word
		types.iter().fold(name.to_string(), |variant, type_name| witness_name(&variant, &type_name.replace(' ', "_")))
	}

	/// `combine(1.1, 2.2)`: the call of the variant the arguments fit best
	fn resolve_typed_call(&mut self, node: &Node) -> Option<Node> {
		let Node::List(items, bracket, separator) = node.drop_meta() else {
			return None;
		};
		let [head, arguments @ ..] = items.as_slice() else {
			return None;
		};
		let variants = self.overloaded.get(&head.drop_meta().name())?.clone();
		if arguments.is_empty() || !is_call(items, bracket, separator) {
			return None;
		}
		let name = head.drop_meta().name();
		let variants: Vec<Vec<String>> = variants.into_iter().filter(|types| types.len() == arguments.len()).collect();
		let cast: Vec<Option<(Node, String)>> = arguments.iter().map(|argument| self.as_list_type(argument)).collect();
		let declared: Vec<Option<String>> = arguments
			.iter()
			.enumerate()
			.map(|(position, argument)| match &cast[position] {
				Some((_, type_name)) => Some(type_name.clone()),
				None => self.declared_type(argument, position, &variants),
			})
			.collect();
		// the list of `[] as [file]` goes to the variant as it is: the cast picked the variant
		let arguments: Vec<Node> = arguments
			.iter()
			.zip(cast)
			.map(|(argument, cast)| cast.map_or_else(|| argument.clone(), |(list, _)| list))
			.collect();
		let kinds: Vec<Option<Kind>> = arguments
			.iter()
			.zip(&declared)
			.map(|(argument, declared)| match declared.as_deref() {
				Some(type_name) if type_name != ANY_TYPE => {
					overload_kind(type_name).or_else(|| argument_kind(argument, &self.variables))
				}
				_ => argument_kind(argument, &self.variables),
			})
			.collect();
		let mut elements: Vec<Option<String>> = arguments
			.iter()
			.zip(&declared)
			.zip(&kinds)
			.map(|((argument, declared), kind)| match kind {
				Some(Kind::List) => self.element_type(argument, declared.as_deref()),
				_ => None,
			})
			.collect();
		let differing_elements = |position: usize| {
			let mut taken = variants.iter().map(|types| crate::analyzer::list_element_type(&types[position]));
			let first = taken.next().flatten();
			taken.any(|element| element != first)
		};
		// decision list-variant-dispatch: `play([])` fits every list variant and takes the first, saying so
		for (position, argument) in arguments.iter().enumerate() {
			if declared[position].is_none() && is_empty_list(argument) && differing_elements(position) {
				let list_types: Vec<&String> = variants.iter().map(|types| &types[position]).collect();
				let question = format!("`[]` fits every variant of {name}: which list is it?");
				let readings = list_types
					.iter()
					.map(|type_name| reading(type_name, &format!("[] as {}", bracketed(type_name))))
					.collect();
				let asked = Ask::new(TOPIC, question, readings, Fallback::Warning).written("[]").at_node(argument);
				let taken = match ask(&asked) {
					Ok(chosen) => list_types[chosen],
					Err(error) => return Some(error),
				};
				elements[position] = crate::analyzer::list_element_type(taken).map(str::to_string);
			}
		}
		// a list whose element type only the run time knows, where the variants take lists of different elements
		let elements_untold = |position: usize| {
			kinds[position] == Some(Kind::List) && elements[position].is_none() && differing_elements(position)
		};
		// an argument declared `any` (a Node) fits only an `any` variant; an argument of a kind unknown at compile time
		// fits every type for now: the run time tests it
		let fits_argument = |position: usize, type_name: &str| match (declared[position].as_deref(), kinds[position]) {
			(Some(ANY_TYPE), _) => (type_name == ANY_TYPE).then_some(0),
			(_, Some(Kind::List)) => element_fit(elements[position].as_deref(), type_name),
			(_, Some(kind)) => fit(kind, type_name),
			(_, None) => Some(0),
		};
		let is_known = |position: usize| {
			(kinds[position].is_some() && !elements_untold(position)) || declared[position].as_deref() == Some(ANY_TYPE)
		};
		let scores: Vec<(u32, &Vec<String>)> = variants
			.iter()
			.filter_map(|types| {
				let fits: Option<Vec<u32>> =
					types.iter().enumerate().map(|(position, type_name)| fits_argument(position, type_name)).collect();
				Some((fits?.iter().sum(), types))
			})
			.collect();
		let written = node.serialize().trim().to_string();
		let listed =
			variants.iter().map(|types| format!("{name}({})", types.join(", "))).collect::<Vec<_>>().join(", ");
		let no_variant = format!("`{written}` fits no single variant of {listed}");
		let arguments: Vec<Node> = arguments.iter().map(|argument| self.rewrite(argument.clone())).collect();
		if !(0..arguments.len()).all(is_known) && scores.len() > 1 {
			let candidates: Vec<&Vec<String>> = scores.iter().map(|(_, types)| *types).collect();
			// a list of untold elements is tested at run time, as an argument of unknown kind
			let tested_kinds: Vec<Option<Kind>> =
				(0..arguments.len()).map(|position| kinds[position].filter(|_| !elements_untold(position))).collect();
			return Some(Self::dispatched(&name, &candidates, &tested_kinds, arguments, &no_variant));
		}
		let best = scores.iter().map(|(score, _)| *score).min();
		let chosen: Vec<&Vec<String>> =
			scores.iter().filter(|(score, _)| Some(*score) == best).map(|(_, types)| *types).collect();
		match chosen.as_slice() {
			[types] => Some(call(&Self::variant_name(&name, types), arguments)),
			[] => Some(crate::errors::diagnostic::Diagnostic::at(head, no_variant).into_error()),
			_ => Some(
				crate::errors::diagnostic::Diagnostic::at(
					head,
					format!(
						"`{written}` fits no single variant of {listed}: give the arguments a type, e.g. `x as float`"
					),
				)
				.into_error(),
			),
		}
	}

	/// `f(g(), 1)` of an argument whose kind only the run time knows: `(overload·1 = g(); if is_type(overload·1,
	/// "text") then f·text·any(overload·1, 1) else if … else raise "… fits no single variant …")`, the variants testing more
	/// arguments first, else in declaration order; each argument is evaluated once
	fn dispatched(
		name: &str,
		candidates: &[&Vec<String>],
		kinds: &[Option<Kind>],
		arguments: Vec<Node>,
		no_variant: &str,
	) -> Node {
		let mut bindings = vec![];
		let arguments: Vec<Node> = arguments
			.into_iter()
			.zip(kinds)
			.map(|(argument, kind)| match (kind, argument.drop_meta()) {
				(None, Node::List(..) | Node::Key(..)) => super::nodes::bound_once("overload", argument, &mut bindings),
				_ => argument,
			})
			.collect();
		let tested_types = |types: &[String]| -> Vec<(Node, String)> {
			arguments
				.iter()
				.zip(kinds)
				.zip(types)
				.filter(|((_, kind), type_name)| kind.is_none() && *type_name != ANY_TYPE)
				.map(|((argument, _), type_name)| (argument.clone(), type_name.clone()))
				.collect()
		};
		// a variant testing more arguments first, an exact one before one taking its arguments by widening
		let widened =
			|types: &[String]| tested_types(types).iter().filter(|(_, type_name)| takes_widened(type_name)).count();
		let mut ordered = candidates.to_vec();
		ordered.sort_by_key(|types| (std::cmp::Reverse(tested_types(types).len()), widened(types)));
		let refusal = call(crate::words::RAISE_WORD, vec![Node::Text(no_variant.to_string())]);
		let cases = ordered.iter().map(|types| {
			let variant = call(&Self::variant_name(name, types), arguments.clone());
			let tests =
				tested_types(types).into_iter().filter_map(|(argument, type_name)| run_time_test(argument, &type_name));
			(tests.reduce(|all, test| key(all, Op::And, test)), variant)
		});
		super::nodes::after_bindings(bindings, super::nodes::tested_cases(cases.collect(), refusal))
	}
}
