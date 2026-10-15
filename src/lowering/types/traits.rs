//! Traits (also called interfaces, protocols, typeclasses, capabilities, aspects; see notes/traits.md).
//! Built in: Comparable and Equatable, which the built-in kinds conform to as their runtime functions order and compare
//! them. `trait shape{area}` declares one more. A declared type conforms by defining the operations for itself,
//! `compare(a:person, b:person) := a.age - b.age`, `area(s:square) := s.side*s.side` (structural conformance, T2).
//! Such a definition becomes the witness `compare·person`, so every type has its own. Where the static type of an
//! instance is known, `<`, `==` and operation calls call the witness and a missing conformance is a compile error naming
//! the fix; elsewhere node_order asks the runtime witness table (wasm_emitter/witness.rs).

use super::nodes::{call, called_name, children_rewritten, if_then, if_then_else, key};
use crate::analyzer::{call_name, collect_all_types};
use crate::errors::diagnostic::Diagnostic;
use crate::node::{parens, symbol, Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_constructor::{instance_parts, Instance};
use crate::type_kinds::TypeRegistry;
use crate::words::{FOR_WORD, IN_WORD, LIST_WORD, SELF_WORD, SORT_WORD};
use std::cell::RefCell;
use std::collections::HashMap;

mod likeness;
pub use likeness::*;
mod generics;
pub(crate) use generics::*;
mod instance_types;
pub use instance_types::*;
mod dispatch;
pub use dispatch::*;
mod instance_calls;
pub use instance_calls::*;

/// The words that declare a trait: `trait`, the canonical one (user, 2026-10-03: "short"), then the names other languages
/// use, each accepted with a hint toward `trait`
pub const TRAIT_KEYWORDS: [&str; 8] =
	["trait", "interface", "protocol", "typeclass", "prototype", "capability", "aspect", "feature"];
pub const WITNESS_SEPARATOR: char = '·';
/// The parameter name a fix shows for an operation declared without parameters, `trait shape{area}`
const DEFAULT_PARAMETER: &str = "x";
/// Static type names (`type(x)`) of the built-in kinds that `node_order` orders
const BUILTIN_COMPARABLE: [&str; 6] = ["int", "rational", "real", "float", "text", "codepoint"];

pub const COMPARABLE: &str = "Comparable";
/// `image like photo`: an image is usable wherever a photo is expected, judged by the fields it is used with
/// (notes/welcoming.md, "Duck typing and like")
pub const LIKE_WORD: &str = "like";
/// A known type passed where another is expected: the error names both and teaches `like` (GIVEN, DECLARED)
const NOT_LIKE_MESSAGE: &str = "GIVEN is not a DECLARED";
const NOT_LIKE_FIX: &str = "declare `GIVEN like DECLARED` to use it as one (judged by the fields it is used with)";
const LIKE_OF_UNDECLARED: &str = "NAME is not a declared type: `like` relates two classes";
/// Data given for a declared type (an ad hoc name `pic{…}` or a map): its fields are compared with the declared ones
/// (DATA, FIELDS, THEM, DECLARED)
const MISSING_FIELDS_WARNING: &str = "DATA lacks FIELDS of DECLARED";
const MISSING_FIELDS_FIX: &str = "give FIELDS, or declare THEM optional (x?) or with a default in DECLARED";
const EXTRA_FIELDS_WARNING: &str = "DATA has FIELDS, which DECLARED does not declare";
const EXTRA_FIELDS_FIX: &str = "drop FIELDS, or declare THEM in DECLARED";
/// A function named like a field of its parameter's class (NAME, CLASS, PARAMETER)
const FIELD_METHOD_WARNING: &str = "CLASS has a field NAME and a method NAME(PARAMETER:CLASS): PARAMETER.NAME reads the field, PARAMETER.NAME() and NAME(PARAMETER) call the method";
const FIELD_METHOD_FIX: &str = "rename the field or the method";
pub const COMPARE: &str = "compare";
/// T4: every value is Equatable by value; `equals` overrides it for a type
pub const EQUATABLE: &str = "Equatable";
pub const EQUALS: &str = "equals";
/// Callable (wiki trait.md): an instance applied to arguments, `room(signal)`, is `call(room, signal)` of its type
pub const CALL: &str = "call";
/// The operations a program uses without naming them (`room(signal)`, `p < q`, `p == q`): a module's definitions of
/// them for a type the program mentions come along (needed_definitions)
pub const IMPLICIT_OPERATIONS: [&str; 3] = [CALL, COMPARE, EQUALS];

#[derive(Clone, Debug, PartialEq)]
pub struct Trait {
	pub name: String,
	pub operations: Vec<Operation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
	pub name: String,
	/// The parameter names the declaration writes; the first one takes the conforming type
	pub parameters: Vec<String>,
	/// Every parameter takes the conforming type (`compare(a:T, b:T)`), not only the first
	pub symmetric: bool,
	/// What the operation returns, as the fix of a missing conformance explains it
	pub contract: Option<&'static str>,
	/// The body of a default method, `describe(s) := "area " + area(s)`: every conforming type that does not define the
	/// operation gets it (`default_methods`)
	pub default: Option<Node>,
}

impl Operation {
	/// `compare(a:dot, b:dot)`, `area(x:dot)`: the definition a type T needs
	fn signature(&self, type_name: &str) -> String {
		let parameters: Vec<String> = self
			.parameters
			.iter()
			.enumerate()
			.map(
				|(index, parameter)| {
					if index == 0 || self.symmetric {
						format!("{parameter}:{type_name}")
					} else {
						parameter.clone()
					}
				},
			)
			.collect();
		format!("{}({})", self.name, parameters.join(", "))
	}

	fn fix(&self, type_name: &str) -> String {
		match self.contract {
			Some(contract) => format!("define {} := … ({contract})", self.signature(type_name)),
			None => format!("define {} := …", self.signature(type_name)),
		}
	}
}

fn builtin_traits() -> Vec<Trait> {
	let binary = |name: &str, contract| Operation {
		name: name.to_string(),
		parameters: vec!["a".into(), "b".into()],
		symmetric: true,
		contract: Some(contract),
		default: None,
	};
	vec![
		Trait { name: COMPARABLE.to_string(), operations: vec![binary(COMPARE, "negative, 0 or positive")] },
		Trait { name: EQUATABLE.to_string(), operations: vec![binary(EQUALS, "true when equal")] },
	]
}

pub fn is_builtin_trait(name: &str) -> bool {
	name == COMPARABLE || name == EQUATABLE
}

/// The function that implements `operation` for the declared type `type_name`
pub fn witness_name(operation: &str, type_name: &str) -> String {
	format!("{operation}{WITNESS_SEPARATOR}{type_name}")
}

/// The operation of a witness name, `agent` for `agent·1`; a name that is none is its own
pub fn witness_operation(name: &str) -> &str {
	name.split_once(WITNESS_SEPARATOR).map_or(name, |(operation, _)| operation)
}

/// The declared type of a witness name, `person` for `compare·person`
pub fn witness_type<'a>(name: &'a str, operation: &str) -> Option<&'a str> {
	name.strip_prefix(operation)?.strip_prefix(WITNESS_SEPARATOR)
}

/// Does a value of the built-in static type `actual` (`type(x)`) conform to the built-in trait named `spec`?
pub fn builtin_conforms(actual: &str, spec: &str) -> Option<bool> {
	match spec {
		COMPARABLE => Some(BUILTIN_COMPARABLE.contains(&actual)),
		EQUATABLE => Some(true),
		_ => None,
	}
}

/// The built-in traits and the ones the program declares
pub struct Traits(Vec<Trait>);

impl Traits {
	pub fn of(node: &Node) -> Self {
		let mut traits = builtin_traits();
		collect_declarations(node, &mut traits);
		Traits(traits)
	}

	pub fn is_trait(&self, name: &str) -> bool {
		self.named(name).is_some()
	}

	/// Is `name` an operation of a trait (a declared one, a built-in one, a class method several classes share)
	pub fn is_operation(&self, name: &str) -> bool {
		self.operation(name).is_some()
	}

	fn named(&self, name: &str) -> Option<&Trait> {
		self.0.iter().find(|candidate| candidate.name == name)
	}

	/// The trait the elements of a list annotation need, Comparable of `xs: Comparable list` (`list of Comparable`)
	fn element_trait(&self, type_name: &str) -> Option<&Trait> {
		self.named(type_name.strip_prefix(crate::analyzer::LIST_OF_PREFIX)?)
	}

	fn operation(&self, name: &str) -> Option<(&Trait, &Operation)> {
		self.0.iter().find_map(|candidate| {
			Some((candidate, candidate.operations.iter().find(|operation| operation.name == name)?))
		})
	}

	/// The first operation of the trait that the type does not define; Equatable is synthesized for every type
	fn missing<'a>(&'a self, required: &'a Trait, type_name: &str, witnesses: &[String]) -> Option<&'a Operation> {
		if required.name == EQUATABLE {
			return None;
		}
		required.operations.iter().find(|operation| !witnesses.contains(&witness_name(&operation.name, type_name)))
	}
}

fn missing_conformance(
	located: &Node,
	type_name: &str,
	required: &Trait,
	operation: &Operation,
	needed_by: &str,
) -> Node {
	let message = format!("{type_name} is not {}: {needed_by} needs {}", required.name, operation.signature(type_name));
	Diagnostic::at(located, message).fix(operation.fix(type_name)).into_error()
}

// ═══════════════════════════════════════════════════════════════════════════
// Declarations: `trait shape{area perimeter}`
// ═══════════════════════════════════════════════════════════════════════════

/// A declaration stays in the program as ø carrying the trait, which every later pass reads (Traits::of)
/// A parameter typed by a declared trait, `f(s:Shape)`, untyped before class bodies are lowered (class_methods reads
/// typed parameters as instances of a class)
pub fn lower_trait_parameters(node: Node) -> Node {
	let mut names = vec![];
	node.visit(&mut |part| names.extend(declared_trait_name(part)));
	if names.is_empty() {
		node
	} else {
		without_trait_parameter_types(node, &names)
	}
}

/// The name a trait declaration declares, `Shape` of `trait Shape{…}` and `interface Shape {…}`
fn declared_trait_name(node: &Node) -> Option<String> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let (keyword, name) = match items.as_slice() {
		[keyword, body] => (
			keyword,
			match body.drop_meta() {
				Node::Key(name, Op::Colon | Op::None, _) => name.as_ref(),
				_ => return None,
			},
		),
		[keyword, name, block] if matches!(block.drop_meta(), Node::List(_, Bracket::Curly, _)) => (keyword, name),
		_ => return None,
	};
	let is_keyword = TRAIT_KEYWORDS.contains(&keyword.drop_meta().name().as_str());
	(is_keyword && matches!(name.drop_meta(), Node::Symbol(_))).then(|| name.drop_meta().name())
}

pub fn lower_declarations(node: Node) -> Node {
	let mut errors = vec![];
	let node = declare(node, &mut errors);
	if let Some(error) = errors.into_iter().next() {
		return error;
	}
	let mut declared = vec![];
	collect_declarations(&node, &mut declared);
	let node = with_default_methods(node, &declared);
	let names: Vec<String> = declared.into_iter().map(|declared| declared.name).collect();
	// again for the definitions later passes made (Go's `func total(s Shape) float64 {…}` is `total(s:Shape)` now)
	if names.is_empty() {
		node
	} else {
		without_trait_parameter_types(with_conformance_tests(node, &names), &names)
	}
}

/// `f(s:Shape) := s.area()`: a parameter typed by a trait takes any conforming value, untyped; its operations pick the
/// witness of the value's type (a dispatcher where several types define them)
fn without_trait_parameter_types(node: Node, names: &[String]) -> Node {
	match node {
		Node::Key(head, Op::Define, body) => {
			let head = match *head {
				Node::Key(call, Op::Colon, result) => {
					Node::Key(Box::new(untyped_call(*call, names)), Op::Colon, result)
				}
				call => untyped_call(call, names),
			};
			key(head, Op::Define, without_trait_parameter_types(*body, names))
		}
		other => other.map_children(|child| without_trait_parameter_types(child, names)),
	}
}

/// The call head `f(s:Shape, n)` as `f(s, n)` when Shape is one of the trait `names`
fn untyped_call(call: Node, names: &[String]) -> Node {
	let untyped = |parameter: Node| match parameter.drop_meta() {
		Node::Key(name, Op::Colon, trait_name) if matches!(trait_name.drop_meta(), Node::Symbol(word) if names.contains(word)) => {
			name.as_ref().clone()
		}
		_ => parameter,
	};
	match call.drop_meta().clone() {
		Node::List(items, Bracket::Round, separator) if !items.is_empty() => {
			// `xs: shape list` keeps its type: the list annotation reads it (analyzer::with_list_annotation)
			let is_list_word = |index: usize| items.get(index).is_some_and(|next| next.drop_meta().name() == LIST_WORD);
			let parameters = items.iter().enumerate().skip(1).map(|(index, parameter)| {
				if is_list_word(index + 1) {
					parameter.clone()
				} else {
					untyped(parameter.clone())
				}
			});
			Node::List(items[..1].iter().cloned().chain(parameters).collect(), Bracket::Round, separator)
		}
		_ => call,
	}
}

/// The program with the default methods of its traits defined for every type that defines the other operations of the
/// trait but not the default one: `describe(s:square) := "area " + area(s)`
fn with_default_methods(node: Node, traits: &[Trait]) -> Node {
	let mut defined: Vec<(String, String)> = vec![];
	collect_typed_definitions(&node, &mut defined);
	let mut definitions = vec![];
	for declared in traits {
		let types: Vec<&String> = defined.iter().map(|(_, type_name)| type_name).collect();
		for type_name in types.into_iter().collect::<std::collections::BTreeSet<_>>() {
			let defines = |operation: &Operation| defined.contains(&(operation.name.clone(), type_name.clone()));
			let conforms = declared.operations.iter().filter(|operation| operation.default.is_none()).all(defines);
			for operation in declared.operations.iter().filter(|operation| conforms && !defines(operation)) {
				let Some(body) = &operation.default else { continue };
				let parameters = operation.parameters.iter().enumerate().map(|(index, parameter)| match index {
					0 => key(Node::Symbol(parameter.clone()), Op::Colon, Node::Symbol(type_name.clone())),
					_ => Node::Symbol(parameter.clone()),
				});
				let head = parens(std::iter::once(Node::Symbol(operation.name.clone())).chain(parameters).collect());
				definitions.push(key(head, Op::Define, body.clone()));
			}
		}
	}
	if definitions.is_empty() {
		return node;
	}
	definitions.push(node);
	Node::List(definitions, Bracket::None, Separator::Semicolon)
}

/// (operation, type) of every definition whose first parameter is typed: `area(s:square) := …` is (area, square)
fn collect_typed_definitions(node: &Node, defined: &mut Vec<(String, String)>) {
	node.visit(&mut |part| {
		let Node::Key(head, Op::Define | Op::Assign, _) = part else {
			return;
		};
		let Node::List(items, Bracket::Round, _) = head.drop_meta() else {
			return;
		};
		let [name, first, ..] = items.as_slice() else { return };
		let (Node::Symbol(name), Node::Key(_, Op::Colon, type_node)) = (name.drop_meta(), first.drop_meta()) else {
			return;
		};
		defined.push((name.clone(), type_node.drop_meta().name()));
	});
}

fn collect_declarations(node: &Node, traits: &mut Vec<Trait>) {
	collect_declared(node, traits)
}

/// The values the declarations of the program carry (a Trait, a Likeness), in order
fn collect_declared<T: Clone + 'static>(node: &Node, found: &mut Vec<T>) {
	match node {
		Node::Meta { data, node } => match data.as_ref() {
			Node::Data(dada) if dada.downcast_ref::<T>().is_some() => {
				found.push(dada.downcast_ref::<T>().expect("checked").clone())
			}
			_ => collect_declared(node, found),
		},
		Node::Key(left, _, right) => {
			collect_declared(left, found);
			collect_declared(right, found);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| collect_declared(item, found)),
		_ => {}
	}
}

/// `trait shape{area perimeter}`, `interface shape{area(s)}`: the trait, or the error of an operation it cannot take yet
fn declaration(node: &Node) -> Option<Result<Trait, Node>> {
	let Node::List(items, _, _) = node.drop_meta() else {
		return None;
	};
	let (keyword, name, requirements) = match items.as_slice() {
		[keyword, body] => match body.drop_meta() {
			Node::Key(name, Op::Colon | Op::None, requirements) => (keyword, name.as_ref(), requirements.as_ref()),
			_ => return None,
		},
		// `trait Shape { area }` spaced
		[keyword, name, requirements] if matches!(requirements.drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			(keyword, name, requirements)
		}
		_ => return None,
	};
	let keyword_name = keyword.drop_meta().name();
	if !TRAIT_KEYWORDS.contains(&keyword_name.as_str()) {
		return None;
	}
	let Node::Symbol(name) = name.drop_meta() else {
		return None;
	};
	if keyword_name != TRAIT_KEYWORDS[0] {
		let written = format!("{keyword_name} {name}{{…}}");
		crate::normalize::set_position_of(node);
		crate::normalize::hint(&written, &format!("{} {name}{{…}}", TRAIT_KEYWORDS[0]), "warp calls it a trait");
	}
	if is_builtin_trait(name) {
		return Some(Err(Diagnostic::at(keyword, format!("{name} is a built-in trait"))
			.fix(format!("name your trait otherwise than {name}"))
			.into_error()));
	}
	let requirements = match requirements.drop_meta() {
		// Kotlin's `fun area(): Int`, Swift's `func area() -> Double` on one line: the keyword stands apart from the signature
		Node::List(items, Bracket::Curly, _) => items
			.iter()
			.filter(
				|item| !matches!(item.drop_meta(), Node::Symbol(word) if crate::operators::is_function_keyword(word)),
			)
			.cloned()
			.collect(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	};
	let operations = requirements.iter().map(|requirement| {
		operation(requirement)
			.or_else(|| default_method(requirement))
			.or_else(|| foreign_signature(requirement))
			.or_else(|| function_type_member(requirement))
			.ok_or_else(|| {
				let message =
					format!("trait {name} takes operations like `area` or `area(s)`, got {}", requirement.serialize());
				Diagnostic::at(requirement, message)
					.fix("define the operation for each type that conforms, e.g. area(s:square) := …")
					.into_error()
			})
	});
	Some(operations.collect::<Result<Vec<_>, _>>().map(|operations| Trait { name: name.clone(), operations }))
}

fn operation(requirement: &Node) -> Option<Operation> {
	let (name, parameters) = match requirement.drop_meta() {
		Node::Symbol(name) => (name.clone(), vec![DEFAULT_PARAMETER.to_string()]),
		Node::List(items, Bracket::Round, _) => {
			let Node::Symbol(name) = items.first()?.drop_meta() else {
				return None;
			};
			let parameters: Option<Vec<String>> =
				items[1..].iter().map(|parameter| parameter.symbol_name().map(String::from)).collect();
			(name.clone(), parameters.filter(|parameters| !parameters.is_empty())?)
		}
		_ => return None,
	};
	Some(Operation { name, parameters, symmetric: false, contract: None, default: None })
}

/// A method signature as other languages write it in an interface, its receiver implicit: Java's `double area()`,
/// Kotlin's `fun area(): Int`, Swift's `func area() -> Double`, Go's `Area() float64`, TypeScript's `area(): number`,
/// Rust's `fn area(&self) -> f64`: the operation `area(x)` (the result type and parameter types dropped)
fn foreign_signature(requirement: &Node) -> Option<Operation> {
	let (name, parameters) = signature_call(requirement)?;
	let is_receiver = |parameter: &Node| {
		parameter.is_symbol(SELF_WORD)
			|| matches!(parameter.drop_meta(), Node::Key(_, _, word) if word.drop_meta().name() == SELF_WORD)
	};
	let names =
		parameters.iter().filter(|parameter| !is_receiver(parameter)).map(|parameter| match parameter.drop_meta() {
			Node::Key(name, Op::Colon, _) => Some(name.drop_meta().name()),
			Node::Symbol(name) => Some(name.clone()),
			// Java's `int n`, Go's `n int`: the word that is no type
			Node::List(words, _, _) => words
				.iter()
				.map(|word| word.drop_meta().name())
				.find(|word| crate::analyzer::type_word_kind(word).is_none()),
			_ => None,
		});
	let names: Vec<String> =
		std::iter::once(Some(DEFAULT_PARAMETER.to_string())).chain(names).collect::<Option<_>>()?;
	Some(Operation { name, parameters: names, symmetric: false, contract: None, default: None })
}

/// A member typed as a function, as WIT and TypeScript write an interface: `add: (i32, i32) -> i32` is the operation
/// `add(p1, p2)` (the types dropped, as in foreign_signature)
fn function_type_member(requirement: &Node) -> Option<Operation> {
	let Node::Key(member, Op::Arrow, _) = requirement.drop_meta() else {
		return None;
	};
	let Node::Key(name, Op::Colon, parameter_types) = member.drop_meta() else {
		return None;
	};
	let Node::Symbol(name) = name.drop_meta() else {
		return None;
	};
	let count = match parameter_types.drop_meta() {
		Node::List(types, Bracket::Round, _) => types.len(),
		Node::Empty => 0,
		_ => 1,
	};
	let parameters = (1..=count).map(|position| format!("p{position}")).collect();
	Some(Operation { name: name.clone(), parameters, symmetric: false, contract: None, default: None })
}

/// The name and parameters of the first call in a signature, through its keyword, result type and modifiers
fn signature_call(node: &Node) -> Option<(String, Vec<Node>)> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) => match items.split_first()?.0.drop_meta() {
			Node::Symbol(name) => Some((
				name.clone(),
				items[1..]
					.iter()
					.flat_map(|parameter| match parameter.drop_meta() {
						Node::List(group, Bracket::Round, _) => group.clone(),
						_ => vec![parameter.clone()],
					})
					.collect(),
			)),
			_ => None,
		},
		Node::List(items, _, _) => items.iter().find_map(signature_call),
		Node::Key(left, Op::Colon | Op::Arrow, _) => signature_call(left),
		_ => None,
	}
}

/// `describe(s) := body` inside a trait: the operation with its default body
fn default_method(requirement: &Node) -> Option<Operation> {
	let Node::Key(head, Op::Define | Op::Assign, body) = requirement.drop_meta() else {
		return None;
	};
	Some(Operation { default: Some(body.as_ref().clone()), ..operation(head)? })
}

/// `x is shape` for a declared trait is a type test, as `x is Comparable`
fn with_conformance_tests(node: Node, names: &[String]) -> Node {
	match node {
		Node::Key(subject, Op::Eq, right) if matches!(right.drop_meta(), Node::Symbol(name) if names.contains(name)) => {
			let subject = with_conformance_tests(*subject, names);
			call(crate::words::IS_TYPE, vec![subject, Node::Text(right.drop_meta().name())])
		}
		other => children_rewritten(other, |child| with_conformance_tests(child, names)),
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Conformances: witnesses and claims
// ═══════════════════════════════════════════════════════════════════════════

/// Rename every definition `compare(a:T, b:T)` of a declared type T to its witness, mark parameters of a declared type,
/// and check explicit claims `class T{…} is Comparable`
pub fn lower_conformances(node: Node) -> Node {
	let mut registry = TypeRegistry::new();
	collect_all_types(&mut registry, &node);
	if registry.types().is_empty() {
		return node;
	}
	let traits = Traits::of(&node);
	if let Err(error) = crate::errors::diagnostic::report(&field_method_warnings(&node, &registry)) {
		return error;
	}
	let generics = generic_functions(&node, &registry, &traits);
	let node = conform(operation_calls(node, &registry, &traits, &generics), &registry, &traits, &generics);
	let node = match generics.is_empty() {
		true => node,
		false => match generic_calls(node, generics) {
			Ok(node) => node,
			Err(error) => return error,
		},
	};
	let witnesses = defined_functions(&node);
	let node = witness_references(node, &witnesses);
	match first_unkept_claim(&node, &traits, &witnesses) {
		Some(error) => error,
		None => without_claims(node, &traits),
	}
}

/// `s.area()` of a declared trait's operation or of a generic function: the call `area(s)`, which picks the witness of
/// s's type like any call of the operation (a class method `area(): number {…}` is the witness area·Square); an uncalled
/// `b.size` of a declared field reads the field
fn operation_calls(node: Node, registry: &TypeRegistry, traits: &Traits, generics: &HashMap<String, Generic>) -> Node {
	let is_operation = |member: &Node| {
		let name = crate::node::leading_name(member);
		let reads_field = matches!(member.drop_meta(), Node::Symbol(_)) && is_declared_field(registry, &name);
		!reads_field
			&& (traits.operation(&name).is_some_and(|(declared, _)| !is_builtin_trait(&declared.name))
				|| generics.contains_key(&name))
	};
	match node {
		Node::Key(receiver, Op::Dot, member) if is_operation(&member) => {
			let arguments = match member.drop_meta() {
				Node::List(items, _, _) => items[1..].to_vec(),
				_ => vec![],
			};
			let name = Node::Symbol(crate::node::leading_name(&member));
			let arguments = std::iter::once(*receiver)
				.chain(arguments)
				.map(|argument| operation_calls(argument, registry, traits, generics));
			parens(std::iter::once(name).chain(arguments).collect())
		}
		other => other.map_children(|child| operation_calls(child, registry, traits, generics)),
	}
}

fn is_declared_field(registry: &TypeRegistry, name: &str) -> bool {
	registry.types().iter().any(|type_def| type_def.fields.iter().any(|field| field.name == name))
}

/// A warning per definition `size(b:Box)` where Box has a field size: `b.size` and `b.size()` then mean different things
fn field_method_warnings(node: &Node, registry: &TypeRegistry) -> Vec<Diagnostic> {
	let mut warnings = vec![];
	node.visit(&mut |child| {
		let Node::Key(head, Op::Assign | Op::Define, _) = child else {
			return;
		};
		if let Some(items) = definition_head(head) {
			let name = items[0].name();
			for (parameter, class) in instance_parameters(head, registry) {
				if registry
					.get_by_name(&class)
					.is_some_and(|type_def| type_def.fields.iter().any(|field| field.name == name))
				{
					let fill = |template: &str| {
						template.replace("NAME", &name).replace("CLASS", &class).replace("PARAMETER", &parameter)
					};
					warnings.push(Diagnostic::at(head, fill(FIELD_METHOD_WARNING)).fix(fill(FIELD_METHOD_FIX)));
				}
			}
		}
	});
	warnings
}

fn conform(node: Node, registry: &TypeRegistry, traits: &Traits, generics: &HashMap<String, Generic>) -> Node {
	match node {
		Node::Key(head, op @ (Op::Assign | Op::Define), body) if definition_head(&head).is_some() => {
			let body = instance_parameters(&head, registry)
				.iter()
				.fold(*body, |body, (name, type_name)| typed_uses(body, name, type_name));
			let head = typed_parameters(*head, registry);
			let head = match witness_of(&head, registry, traits).or_else(|| method_of(&head, registry, generics)) {
				Some(witness) => renamed_head(head, witness),
				None => head,
			};
			key(head, op, conform(body, registry, traits, generics))
		}
		other => children_rewritten(other, |child| conform(child, registry, traits, generics)),
	}
}

/// The items of a definition head `f(a:T, b)`
fn definition_head(head: &Node) -> Option<&[Node]> {
	match head.drop_meta() {
		Node::List(items, _, _)
			if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) && items.len() > 1 =>
		{
			Some(items)
		}
		_ => None,
	}
}

/// `f()`: the head of a definition without parameters, whose body has variables of its own
fn is_parameterless_head(head: &Node) -> bool {
	matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if matches!(items.as_slice(), [name] if matches!(name.drop_meta(), Node::Symbol(_))))
}

/// The parameters of a definition head written without a type: `p` and `p=default` in `f(p, q=1)`, the implicit `it` of
/// a head without parameters (`f := it.width`)
pub fn untyped_parameters(head: &Node) -> Vec<String> {
	if matches!(head.drop_meta(), Node::Symbol(_)) {
		return vec![crate::words::IMPLICIT_PARAMETER.to_string()];
	}
	let parameters = definition_head(head).map(|items| &items[1..]).unwrap_or_default();
	let untyped = |parameter: &Node| match parameter.drop_meta() {
		Node::Symbol(name) => Some(name.clone()),
		Node::Key(name, Op::Assign, _) => Some(name.name()),
		_ => None,
	};
	parameters.iter().filter_map(untyped).collect()
}

/// The declared type a parameter `name:T` names
fn parameter_type<'a>(parameter: &'a Node, registry: &TypeRegistry) -> Option<&'a str> {
	let Node::Key(_, Op::Colon, type_node) = parameter.drop_meta() else {
		return None;
	};
	let Node::Symbol(type_name) = type_node.drop_meta() else {
		return None;
	};
	registry.get_by_name(type_name).map(|_| type_name.as_str())
}

/// The parameters `name:T` of a definition head that name a declared type T
fn instance_parameters(head: &Node, registry: &TypeRegistry) -> Vec<(String, String)> {
	let parameters = definition_head(head).map(|items| &items[1..]).unwrap_or_default();
	let name_and_type = |parameter: &Node| match parameter.drop_meta() {
		Node::Key(name, Op::Colon, _) => Some((name.name(), parameter_type(parameter, registry)?.to_string())),
		_ => None,
	};
	parameters.iter().filter_map(name_and_type).collect()
}

/// Marks a use of an instance parameter with its declared type, which InstanceTypes reads (scoped to the definition)
#[derive(Clone, Debug, PartialEq)]
pub struct TypedAs(pub String);

fn typed_uses(node: Node, parameter: &str, type_name: &str) -> Node {
	match node {
		Node::Symbol(name) if name == parameter => {
			Node::meta(Node::Symbol(name), Node::data(TypedAs(type_name.to_string())))
		}
		other => children_rewritten(other, |child| typed_uses(child, parameter, type_name)),
	}
}

fn typed_as(node: &Node) -> Option<String> {
	match node {
		Node::Meta { data, node } => match data.as_ref() {
			Node::Data(dada) => {
				dada.downcast_ref::<TypedAs>().map(|TypedAs(type_name)| type_name.clone()).or_else(|| typed_as(node))
			}
			_ => typed_as(node),
		},
		_ => None,
	}
}

/// `a:person` is an instance parameter: its annotation carries the Instance mark, which analyzer::param_kind reads
fn typed_parameters(head: Node, registry: &TypeRegistry) -> Node {
	let Node::List(items, bracket, separator) = head.drop_meta().clone() else {
		return head;
	};
	let marked = |item: Node| match item.drop_meta() {
		Node::Key(name, Op::Colon, type_node) if parameter_type(&item, registry).is_some() => Node::Key(
			name.clone(),
			Op::Colon,
			Box::new(Node::meta(type_node.drop_meta().clone(), Node::data(Instance))),
		),
		_ => item,
	};
	Node::List(items.into_iter().map(marked).collect(), bracket, separator)
}

/// `area(s:T)`, `compare(a:T, b:T)`: the witness name of a trait operation whose first parameter (every parameter of a
/// symmetric one) takes a declared type T
fn witness_of(head: &Node, registry: &TypeRegistry, traits: &Traits) -> Option<String> {
	let items = definition_head(head)?;
	let Node::Symbol(name) = items[0].drop_meta() else {
		return None;
	};
	let (_, operation) = traits.operation(name)?;
	let parameters = &items[1..];
	let type_name = parameter_type(&parameters[0], registry)?;
	let fits = !operation.symmetric
		|| (parameters.len() == operation.parameters.len()
			&& parameters.iter().all(|parameter| parameter_type(parameter, registry) == Some(type_name)));
	fits.then(|| witness_name(name, type_name))
}

fn renamed_head(head: Node, witness: String) -> Node {
	let Node::List(mut items, bracket, separator) = head else {
		return head;
	};
	items[0] = Node::Symbol(witness);
	Node::List(items, bracket, separator)
}

/// A closure of a method that became a witness makes the witness: `sum` of P's `P.methods` is `sum·P`
fn witness_references(node: Node, witnesses: &[String]) -> Node {
	if let Some((method, class)) = crate::closures::class_method(&node) {
		let witness = witness_name(&method, &class);
		if witnesses.contains(&witness) {
			return crate::closures::class_method_target(&witness, Some(class));
		}
	}
	children_rewritten(node, |child| witness_references(child, witnesses))
}

fn defined_functions(node: &Node) -> Vec<String> {
	let context = crate::analyzer::function_definitions(node);
	context.user_functions.into_keys().collect()
}

/// `class dot{x:int} is Comparable` is the type test of a declaration: a claim
fn claim<'a>(node: &'a Node, traits: &'a Traits) -> Option<(&'a Node, &'a Trait)> {
	let Node::List(items, bracket, separator) = node.drop_meta() else {
		return None;
	};
	if call_name(items, bracket, separator) != Some(crate::words::IS_TYPE) {
		return None;
	}
	let [_, declaration, spec] = items.as_slice() else {
		return None;
	};
	let Node::Text(spec) = spec.drop_meta() else {
		return None;
	};
	matches!(declaration.drop_meta(), Node::Type { .. }).then_some(())?;
	Some((declaration, traits.named(spec)?))
}

/// T2 in one place: a claim is optional, but a claimed trait needs its operations
fn claim_error(declaration: &Node, claimed: &Trait, traits: &Traits, witnesses: &[String]) -> Option<Node> {
	let Node::Type { name, .. } = declaration.drop_meta() else {
		return None;
	};
	let type_name = name.name();
	let operation = traits.missing(claimed, &type_name, witnesses)?;
	let message = format!("{type_name} claims {} but defines no {}", claimed.name, operation.signature(&type_name));
	Some(Diagnostic::at(declaration, message).fix(operation.fix(&type_name)).into_error())
}

/// P177: `class Square implements Shape {…}`, `struct Square: Shape {…}`: the declared traits the parser put on the name
fn named_claims<'a>(node: &'a Node, traits: &'a Traits) -> Vec<&'a Trait> {
	let Node::Type { name, .. } = node.drop_meta() else {
		return vec![];
	};
	let Some(Node::List(claimed, _, _)) = name.attribute(crate::warp_parser::IMPLEMENTS_WORD).map(Node::drop_meta)
	else {
		return vec![];
	};
	claimed.iter().filter_map(|claimed| traits.named(&claimed.drop_meta().name())).collect()
}

fn first_unkept_claim(node: &Node, traits: &Traits, witnesses: &[String]) -> Option<Node> {
	if let Some((declaration, claimed)) = claim(node, traits) {
		return claim_error(declaration, claimed, traits, witnesses);
	}
	if let Some(error) =
		named_claims(node, traits).into_iter().find_map(|claimed| claim_error(node, claimed, traits, witnesses))
	{
		return Some(error);
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => {
			first_unkept_claim(left, traits, witnesses).or_else(|| first_unkept_claim(right, traits, witnesses))
		}
		Node::List(items, _, _) => items.iter().find_map(|item| first_unkept_claim(item, traits, witnesses)),
		_ => None,
	}
}

fn without_claims(node: Node, traits: &Traits) -> Node {
	if let Some((declaration, _)) = claim(&node, traits) {
		return declaration.clone();
	}
	children_rewritten(node, |child| without_claims(child, traits))
}

/// Every function's parameters with the declared type each one takes (a class, a list of a trait), for the
/// functions that take any
fn instance_signatures(
	node: &Node,
	registry: &TypeRegistry,
	traits: &Traits,
) -> HashMap<String, Vec<(String, Option<String>)>> {
	let context = crate::analyzer::function_definitions(node);
	let declared = |param: &crate::compiler::context::Param| {
		param
			.annotation
			.as_ref()
			.map(Node::name)
			.filter(|type_name| names_classes(type_name, registry) || traits.element_trait(type_name).is_some())
	};
	let signature = |function: &crate::compiler::context::UserFunctionDef| {
		function.params.iter().map(|param| (param.name.clone(), declared(param))).collect::<Vec<_>>()
	};
	context
		.user_functions
		.values()
		.map(|function| (function.name.clone(), signature(function)))
		.filter(|(_, params)| params.iter().any(|(_, declared)| declared.is_some()))
		.collect()
}

/// `(found = 0; at = 0; for item in xs { at = at + 1; if found == 0 and equals·T(item, x) != 0 { found = at } }; found)`:
/// the 1-based position of the first item equal to x, 0 when there is none
fn position_by_equals(list: &Node, element: &Node, type_name: &str) -> Node {
	let symbol = |name: &str| symbol(name);
	let (found, at, item) = (symbol(FOUND_VARIABLE), symbol(POSITION_VARIABLE), symbol(ITEM_VARIABLE));
	let assign = |target: &Node, value: Node| key(target.clone(), Op::Assign, value);
	let equal = key(witness_call(EQUALS, type_name, vec![item.clone(), element.clone()]), Op::Ne, Node::int(0));
	let first = key(found.clone(), Op::Eq, Node::int(0));
	let found_it = if_then(
		key(first, Op::And, equal),
		Node::List(vec![assign(&found, at.clone())], Bracket::Curly, Separator::None),
	);
	let step = assign(&at, key(at.clone(), Op::Add, Node::int(1)));
	let body = Node::List(vec![step, found_it], Bracket::Curly, Separator::Semicolon);
	let search =
		Node::List(vec![symbol(FOR_WORD), item, symbol(IN_WORD), list.clone(), body], Bracket::None, Separator::Space);
	Node::List(
		vec![assign(&found, Node::int(0)), assign(&at, Node::int(0)), search, found],
		Bracket::Round,
		Separator::Semicolon,
	)
}
const FOUND_VARIABLE: &str = "equals_found";
const ITEM_VARIABLE: &str = "equals_item";
const POSITION_VARIABLE: &str = "equals_position";

fn witness_call(operation: &str, type_name: &str, arguments: Vec<Node>) -> Node {
	call(&witness_name(operation, type_name), arguments)
}
