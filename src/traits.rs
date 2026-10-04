//! Traits (also called interfaces, protocols, typeclasses, capabilities, aspects; see notes/traits.md).
//! Built in: Comparable and Equatable, which the built-in kinds conform to as their runtime functions order and compare
//! them. `trait shape{area}` declares one more. A declared type conforms by defining the operations for itself,
//! `compare(a:person, b:person) := a.age - b.age`, `area(s:square) := s.side*s.side` (structural conformance, T2).
//! Such a definition becomes the witness `compare·person`, so every type has its own. Where the static type of an
//! instance is known, `<`, `==` and operation calls call the witness and a missing conformance is a compile error naming
//! the fix; elsewhere node_order asks the runtime witness table (wasm_emitter/witness.rs).

use crate::analyzer::{call_name, collect_all_types};
use crate::diagnostic::Diagnostic;
use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::type_constructor::{instance_parts, Instance};
use crate::type_kinds::TypeRegistry;
use std::cell::RefCell;
use std::collections::HashMap;

/// The words that declare a trait: `trait`, the canonical one (user, 2026-10-03: "short"), then the names other languages
/// use, each accepted with a hint toward `trait`
pub const TRAIT_KEYWORDS: [&str; 8] = ["trait", "interface", "protocol", "typeclass", "prototype", "capability", "aspect", "feature"];
const WITNESS_SEPARATOR: char = '·';
const SORT_WORD: &str = "sort";
const FOR_WORD: &str = "for";
const IN_WORD: &str = "in";
/// The annotation `xs: T list` (analyzer::with_list_annotation)
const LIST_OF_PREFIX: &str = "list of ";
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
pub const COMPARE: &str = "compare";
/// T4: every value is Equatable by value; `equals` overrides it for a type
pub const EQUATABLE: &str = "Equatable";
pub const EQUALS: &str = "equals";

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
		let parameters: Vec<String> = self.parameters.iter().enumerate().map(|(index, parameter)| {
			if index == 0 || self.symmetric { format!("{parameter}:{type_name}") } else { parameter.clone() }
		}).collect();
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
	let binary = |name: &str, contract| Operation { name: name.to_string(), parameters: vec!["a".into(), "b".into()], symmetric: true, contract: Some(contract), default: None };
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

	fn named(&self, name: &str) -> Option<&Trait> {
		self.0.iter().find(|candidate| candidate.name == name)
	}

	fn operation(&self, name: &str) -> Option<(&Trait, &Operation)> {
		self.0.iter().find_map(|candidate| Some((candidate, candidate.operations.iter().find(|operation| operation.name == name)?)))
	}

	/// The first operation of the trait that the type does not define; Equatable is synthesized for every type
	fn missing<'a>(&'a self, required: &'a Trait, type_name: &str, witnesses: &[String]) -> Option<&'a Operation> {
		if required.name == EQUATABLE {
			return None;
		}
		required.operations.iter().find(|operation| !witnesses.contains(&witness_name(&operation.name, type_name)))
	}
}

fn missing_conformance(located: &Node, type_name: &str, required: &Trait, operation: &Operation, needed_by: &str) -> Node {
	let message = format!("{type_name} is not {}: {needed_by} needs {}", required.name, operation.signature(type_name));
	Diagnostic::at(located, message).fix(operation.fix(type_name)).into_error()
}

// ═══════════════════════════════════════════════════════════════════════════
// Declarations: `trait shape{area perimeter}`
// ═══════════════════════════════════════════════════════════════════════════

/// A declaration stays in the program as ø carrying the trait, which every later pass reads (Traits::of)
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
	if names.is_empty() { node } else { with_conformance_tests(node, &names) }
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
					0 => Node::Key(Box::new(Node::Symbol(parameter.clone())), Op::Colon, Box::new(Node::Symbol(type_name.clone()))),
					_ => Node::Symbol(parameter.clone()),
				});
				let head = Node::List(std::iter::once(Node::Symbol(operation.name.clone())).chain(parameters).collect(), Bracket::Round, Separator::None);
				definitions.push(Node::Key(Box::new(head), Op::Define, Box::new(body.clone())));
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
		let Node::Key(head, Op::Define | Op::Assign, _) = part else { return };
		let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return };
		let [name, first, ..] = items.as_slice() else { return };
		let (Node::Symbol(name), Node::Key(_, Op::Colon, type_node)) = (name.drop_meta(), first.drop_meta()) else { return };
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
			Node::Data(dada) if dada.downcast_ref::<T>().is_some() => found.push(dada.downcast_ref::<T>().expect("checked").clone()),
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

/// `image like photo`: values of `kind` are accepted wherever `like` is expected
#[derive(Clone, Debug, PartialEq)]
pub struct Likeness {
	pub kind: String,
	pub like: String,
}

/// `image like photo` written as a statement
fn like_declaration(node: &Node) -> Option<Likeness> {
	let Node::List(items, _, Separator::Space) = node.drop_meta() else { return None };
	let [Node::Symbol(kind), Node::Symbol(word), Node::Symbol(like)] = items.iter().map(Node::drop_meta).collect::<Vec<_>>()[..] else { return None };
	(word == LIKE_WORD).then(|| Likeness { kind: kind.clone(), like: like.clone() })
}

/// A likeness naming something that is no declared type: the error of the program
fn undeclared_likeness(likenesses: &[Likeness], registry: &TypeRegistry) -> Option<Node> {
	let undeclared = likenesses.iter().flat_map(|likeness| [&likeness.kind, &likeness.like]).find(|name| registry.get_by_name(name).is_none())?;
	Some(crate::node::error(&LIKE_OF_UNDECLARED.replace("NAME", undeclared)))
}

/// Written data given for a declared type, without its ad hoc name: `pic{width:3}` (no `class pic`) and `{width:3}` are the
/// data `{width:3}`, the name is only a label (user, 2026-10-03)
fn written_data(value: &Node, registry: &TypeRegistry) -> Option<Node> {
	if let Node::Key(label, Op::Colon, data) = value.drop_meta() {
		if matches!(label.drop_meta(), Node::Symbol(name) if registry.get_by_name(name).is_none()) && matches!(data.drop_meta(), Node::List(_, Bracket::Curly, _)) {
			return Some(data.as_ref().clone());
		}
	}
	crate::library_words::object_entries(value).map(|_| value.clone())
}

/// The warnings for data whose fields differ from the declared type's: a required field it lacks, a field it has extra
fn field_warnings(value: &Node, data: &Node, declared: &str, registry: &TypeRegistry) -> Vec<Diagnostic> {
	let Some(type_def) = registry.get_by_name(declared) else { return vec![] };
	let entries = crate::library_words::object_entries(data).unwrap_or_default();
	let given: Vec<String> = entries.into_iter().map(|(name, _)| name).filter(|name| !name.starts_with(crate::node::ATTRIBUTE_MARK)).collect();
	let missing: Vec<String> = type_def.fields.iter().filter(|field| registry.is_required(type_def, field) && !given.contains(&field.name)).map(|field| field.name.clone()).collect();
	let extra: Vec<String> = given.iter().filter(|name| !type_def.fields.iter().any(|field| field.name == **name)).cloned().collect();
	let warning = |fields: &[String], message: &str, fix: &str| {
		let (field_word, pronoun) = if fields.len() == 1 { ("field", "it") } else { ("fields", "them") };
		let fill = |template: &str| template.replace("DATA", &value.serialize()).replace("FIELDS", &format!("{field_word} {}", fields.join(", "))).replace("THEM", pronoun).replace("DECLARED", declared);
		(!fields.is_empty()).then(|| Diagnostic::at(value, fill(message)).fix(fill(fix)))
	};
	[warning(&missing, MISSING_FIELDS_WARNING, MISSING_FIELDS_FIX), warning(&extra, EXTRA_FIELDS_WARNING, EXTRA_FIELDS_FIX)].into_iter().flatten().collect()
}

/// `p:photo = …`: the variable and its declared type, when that is a declared type
fn typed_target(target: &Node, registry: &TypeRegistry) -> Option<(String, String)> {
	let Node::Key(name, Op::Colon, type_node) = target.drop_meta() else { return None };
	let (Node::Symbol(name), Node::Symbol(type_name)) = (name.drop_meta(), type_node.drop_meta()) else { return None };
	registry.get_by_name(type_name).map(|_| (name.clone(), type_name.clone()))
}

/// `image is not a photo`, with the fix `declare image like photo`
fn not_like(argument: &Node, needs: &str, given: &str, declared: &str) -> Node {
	let fill = |template: &str| template.replace("GIVEN", given).replace("DECLARED", declared);
	Diagnostic::at(argument, format!("{needs}: {}", fill(NOT_LIKE_MESSAGE))).fix(fill(NOT_LIKE_FIX)).into_error()
}

fn declare(node: Node, errors: &mut Vec<Node>) -> Node {
	if let Some(likeness) = like_declaration(&node) {
		return Node::meta(Node::Empty, Node::data(likeness));
	}
	if let Some(declared) = declaration(&node) {
		return match declared {
			Ok(declared) => Node::meta(Node::Empty, Node::data(declared)),
			Err(error) => {
				errors.push(error.clone());
				error
			}
		};
	}
	match node {
		Node::Key(left, op, right) => Node::Key(Box::new(declare(*left, errors)), op, Box::new(declare(*right, errors))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| declare(item, errors)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(declare(*node, errors)), data },
		other => other,
	}
}

/// `trait shape{area perimeter}`, `interface shape{area(s)}`: the trait, or the error of an operation it cannot take yet
fn declaration(node: &Node) -> Option<Result<Trait, Node>> {
	let Node::List(items, _, _) = node.drop_meta() else { return None };
	let [keyword, body] = items.as_slice() else { return None };
	let keyword_name = keyword.drop_meta().name();
	if !TRAIT_KEYWORDS.contains(&keyword_name.as_str()) {
		return None;
	}
	let Node::Key(name, Op::Colon | Op::None, requirements) = body.drop_meta() else { return None };
	let Node::Symbol(name) = name.drop_meta() else { return None };
	if keyword_name != TRAIT_KEYWORDS[0] {
		let written = format!("{keyword_name} {name}{{…}}");
		crate::normalize::hint(&written, &format!("{} {name}{{…}}", TRAIT_KEYWORDS[0]), "wasp calls it a trait");
	}
	if is_builtin_trait(name) {
		return Some(Err(Diagnostic::at(keyword, format!("{name} is a built-in trait")).fix(format!("name your trait otherwise than {name}")).into_error()));
	}
	let requirements = match requirements.drop_meta() {
		Node::List(items, Bracket::Curly, _) => items.clone(),
		Node::Empty => vec![],
		single => vec![single.clone()],
	};
	let operations = requirements.iter().map(|requirement| operation(requirement).or_else(|| default_method(requirement)).ok_or_else(|| {
		let message = format!("trait {name} takes operations like `area` or `area(s)`, got {}", requirement.serialize());
		Diagnostic::at(requirement, message).fix("define the operation for each type that conforms, e.g. area(s:square) := …").into_error()
	}));
	Some(operations.collect::<Result<Vec<_>, _>>().map(|operations| Trait { name: name.clone(), operations }))
}

fn operation(requirement: &Node) -> Option<Operation> {
	let (name, parameters) = match requirement.drop_meta() {
		Node::Symbol(name) => (name.clone(), vec![DEFAULT_PARAMETER.to_string()]),
		Node::List(items, Bracket::Round, _) => {
			let Node::Symbol(name) = items.first()?.drop_meta() else { return None };
			let parameters: Option<Vec<String>> = items[1..].iter().map(|parameter| match parameter.drop_meta() {
				Node::Symbol(parameter) => Some(parameter.clone()),
				_ => None,
			}).collect();
			(name.clone(), parameters.filter(|parameters| !parameters.is_empty())?)
		}
		_ => return None,
	};
	Some(Operation { name, parameters, symmetric: false, contract: None, default: None })
}

/// `describe(s) := body` inside a trait: the operation with its default body
fn default_method(requirement: &Node) -> Option<Operation> {
	let Node::Key(head, Op::Define | Op::Assign, body) = requirement.drop_meta() else { return None };
	Some(Operation { default: Some(body.as_ref().clone()), ..operation(head)? })
}

/// `x is shape` for a declared trait is a type test, as `x is Comparable`
fn with_conformance_tests(node: Node, names: &[String]) -> Node {
	match node {
		Node::Key(subject, Op::Eq, right) if matches!(right.drop_meta(), Node::Symbol(name) if names.contains(name)) => {
			let subject = with_conformance_tests(*subject, names);
			Node::List(vec![Node::Symbol(crate::type_tests::IS_TYPE.to_string()), subject, Node::Text(right.drop_meta().name())], Bracket::Round, Separator::None)
		}
		Node::Key(left, op, right) => Node::Key(Box::new(with_conformance_tests(*left, names)), op, Box::new(with_conformance_tests(*right, names))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| with_conformance_tests(item, names)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(with_conformance_tests(*node, names)), data },
		other => other,
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
	let node = conform(node, &registry, &traits);
	let witnesses = defined_functions(&node);
	match first_unkept_claim(&node, &traits, &witnesses) {
		Some(error) => error,
		None => without_claims(node, &traits),
	}
}

fn conform(node: Node, registry: &TypeRegistry, traits: &Traits) -> Node {
	match node {
		Node::Key(head, op @ (Op::Assign | Op::Define), body) if definition_head(&head).is_some() => {
			let body = instance_parameters(&head, registry).iter().fold(*body, |body, (name, type_name)| typed_uses(body, name, type_name));
			let head = typed_parameters(*head, registry);
			let head = match witness_of(&head, registry, traits) {
				Some(witness) => renamed_head(head, witness),
				None => head,
			};
			Node::Key(Box::new(head), op, Box::new(conform(body, registry, traits)))
		}
		Node::Key(left, op, right) => Node::Key(Box::new(conform(*left, registry, traits)), op, Box::new(conform(*right, registry, traits))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| conform(item, registry, traits)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(conform(*node, registry, traits)), data },
		other => other,
	}
}

/// The items of a definition head `f(a:T, b)`
fn definition_head(head: &Node) -> Option<&[Node]> {
	match head.drop_meta() {
		Node::List(items, _, _) if matches!(items.first().map(Node::drop_meta), Some(Node::Symbol(_))) && items.len() > 1 => Some(items),
		_ => None,
	}
}

/// The parameters of a definition head written without a type: `p` and `p=default` in `f(p, q=1)`, the implicit `it` of
/// a head without parameters (`f := it.width`)
pub fn untyped_parameters(head: &Node) -> Vec<String> {
	if matches!(head.drop_meta(), Node::Symbol(_)) {
		return vec![crate::lambdas::IMPLICIT_PARAMETER.to_string()];
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
	let Node::Key(_, Op::Colon, type_node) = parameter.drop_meta() else { return None };
	let Node::Symbol(type_name) = type_node.drop_meta() else { return None };
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
		Node::Symbol(name) if name == parameter => Node::meta(Node::Symbol(name), Node::data(TypedAs(type_name.to_string()))),
		Node::Key(left, op, right) => Node::Key(Box::new(typed_uses(*left, parameter, type_name)), op, Box::new(typed_uses(*right, parameter, type_name))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| typed_uses(item, parameter, type_name)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(typed_uses(*node, parameter, type_name)), data },
		other => other,
	}
}

fn typed_as(node: &Node) -> Option<String> {
	match node {
		Node::Meta { data, node } => match data.as_ref() {
			Node::Data(dada) => dada.downcast_ref::<TypedAs>().map(|TypedAs(type_name)| type_name.clone()).or_else(|| typed_as(node)),
			_ => typed_as(node),
		},
		_ => None,
	}
}

/// `a:person` is an instance parameter: its annotation carries the Instance mark, which analyzer::param_kind reads
fn typed_parameters(head: Node, registry: &TypeRegistry) -> Node {
	let Node::List(items, bracket, separator) = head.drop_meta().clone() else { return head };
	let marked = |item: Node| match item.drop_meta() {
		Node::Key(name, Op::Colon, type_node) if parameter_type(&item, registry).is_some() => {
			Node::Key(name.clone(), Op::Colon, Box::new(Node::meta(type_node.drop_meta().clone(), Node::data(Instance))))
		}
		_ => item,
	};
	Node::List(items.into_iter().map(marked).collect(), bracket, separator)
}

/// `area(s:T)`, `compare(a:T, b:T)`: the witness name of a trait operation whose first parameter (every parameter of a
/// symmetric one) takes a declared type T
fn witness_of(head: &Node, registry: &TypeRegistry, traits: &Traits) -> Option<String> {
	let items = definition_head(head)?;
	let Node::Symbol(name) = items[0].drop_meta() else { return None };
	let (_, operation) = traits.operation(name)?;
	let parameters = &items[1..];
	let type_name = parameter_type(&parameters[0], registry)?;
	let fits = !operation.symmetric || (parameters.len() == operation.parameters.len() && parameters.iter().all(|parameter| parameter_type(parameter, registry) == Some(type_name)));
	fits.then(|| witness_name(name, type_name))
}

fn renamed_head(head: Node, witness: String) -> Node {
	let Node::List(mut items, bracket, separator) = head else { return head };
	items[0] = Node::Symbol(witness);
	Node::List(items, bracket, separator)
}

fn defined_functions(node: &Node) -> Vec<String> {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, node);
	context.user_functions.into_keys().collect()
}

/// `class dot{x:int} is Comparable` is the type test of a declaration: a claim
fn claim<'a>(node: &'a Node, traits: &'a Traits) -> Option<(&'a Node, &'a Trait)> {
	let Node::List(items, bracket, separator) = node.drop_meta() else { return None };
	if call_name(items, bracket, separator) != Some(crate::type_tests::IS_TYPE) {
		return None;
	}
	let [_, declaration, spec] = items.as_slice() else { return None };
	let Node::Text(spec) = spec.drop_meta() else { return None };
	matches!(declaration.drop_meta(), Node::Type { .. }).then_some(())?;
	Some((declaration, traits.named(spec)?))
}

/// T2 in one place: a claim is optional, but a claimed trait needs its operations
fn claim_error(declaration: &Node, claimed: &Trait, traits: &Traits, witnesses: &[String]) -> Option<Node> {
	let Node::Type { name, .. } = declaration.drop_meta() else { return None };
	let type_name = name.name();
	let operation = traits.missing(claimed, &type_name, witnesses)?;
	let message = format!("{type_name} claims {} but defines no {}", claimed.name, operation.signature(&type_name));
	Some(Diagnostic::at(declaration, message).fix(operation.fix(&type_name)).into_error())
}

fn first_unkept_claim(node: &Node, traits: &Traits, witnesses: &[String]) -> Option<Node> {
	if let Some((declaration, claimed)) = claim(node, traits) {
		return claim_error(declaration, claimed, traits, witnesses);
	}
	match node.drop_meta() {
		Node::Key(left, _, right) => first_unkept_claim(left, traits, witnesses).or_else(|| first_unkept_claim(right, traits, witnesses)),
		Node::List(items, _, _) => items.iter().find_map(|item| first_unkept_claim(item, traits, witnesses)),
		_ => None,
	}
}

fn without_claims(node: Node, traits: &Traits) -> Node {
	if let Some((declaration, _)) = claim(&node, traits) {
		return declaration.clone();
	}
	match node {
		Node::Key(left, op, right) => Node::Key(Box::new(without_claims(*left, traits)), op, Box::new(without_claims(*right, traits))),
		Node::List(items, bracket, separator) => Node::List(items.into_iter().map(|item| without_claims(item, traits)).collect(), bracket, separator),
		Node::Meta { node, data } => Node::Meta { node: Box::new(without_claims(*node, traits)), data },
		other => other,
	}
}

// ═══════════════════════════════════════════════════════════════════════════
// Static instance types
// ═══════════════════════════════════════════════════════════════════════════

/// What the compiler knows about a value of a declared type
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
	Instance(String),
	ListOf(String),
}

/// The shapes of the program's variables: a variable only ever bound to values of one shape has it (flow-insensitive,
/// like library_words' object variables); parameters `p:person` are instances
pub struct InstanceTypes {
	variables: HashMap<String, Option<Shape>>,
	/// The shapes user functions return, from their bodies: `make(t) := pdf(t)` returns a pdf
	results: HashMap<String, Option<Shape>>,
	registry: TypeRegistry,
	/// The last inference round: a value whose shape is still unknown is unknown for good
	final_round: bool,
}

/// Fixpoint rounds over the assignments: a variable can be assigned from another one defined later in the text
const INFERENCE_ROUNDS: usize = 3;

impl InstanceTypes {
	pub fn of(node: &Node) -> Self {
		let mut registry = TypeRegistry::new();
		collect_all_types(&mut registry, node);
		let mut types = InstanceTypes { variables: HashMap::new(), results: HashMap::new(), registry, final_round: false };
		if types.registry.types().is_empty() {
			return types;
		}
		for round in 1..=INFERENCE_ROUNDS {
			let (mut variables, mut results) = (HashMap::new(), HashMap::new());
			types.final_round = round == INFERENCE_ROUNDS;
			types.collect(node, &mut variables, &mut results);
			(types.variables, types.results) = (variables, results);
		}
		types
	}

	/// A value of an unknown shape adds nothing (it may be an instance the next round recognizes) until the final round,
	/// where it is unknown for good: like a plain value or another shape it makes the variable shapeless (None), so
	/// `s = square(1); s = xs#2` dispatches at run time
	fn bind(variables: &mut HashMap<String, Option<Shape>>, name: &str, shape: Option<Shape>, is_plain: bool) {
		let agreed = match (variables.get(name), shape) {
			(_, None) if is_plain => None,
			(None, None) => return,
			(Some(earlier), None) => earlier.clone(),
			(None, Some(shape)) => Some(shape),
			(Some(earlier), Some(shape)) => earlier.clone().filter(|earlier| *earlier == shape),
		};
		variables.insert(name.to_string(), agreed);
	}

	fn collect(&self, node: &Node, variables: &mut HashMap<String, Option<Shape>>, results: &mut HashMap<String, Option<Shape>>) {
		match node.drop_meta() {
			Node::Key(target, Op::Assign | Op::Define, value) => {
				let is_plain = crate::min_max::is_plain(value) || matches!(value.drop_meta(), Node::Text(_) | Node::Char(_));
				match (target.drop_meta(), definition_head(target)) {
					(Node::Symbol(name), _) => {
						// `p = field_with(p, …)` (a field assignment) keeps the shape of p
						let keeps_shape = is_field_update(name, value);
						let unknown_for_good = self.final_round && !keeps_shape && self.shape(value).is_none();
						Self::bind(variables, name, self.shape(value), is_plain || unknown_for_good)
					}
					// `d:docx = …` of a declared type
					(Node::Key(name, Op::Colon, type_node), _) if self.registry.get_by_name(&type_node.drop_meta().name()).is_some() => {
						Self::bind(variables, &name.name(), Some(Shape::Instance(type_node.drop_meta().name())), false)
					}
					(_, Some(head)) => Self::bind(results, &head[0].name(), self.shape(value), is_plain),
					_ => {}
				}
				self.collect(value, variables, results);
			}
			Node::Key(left, _, right) => {
				self.collect(left, variables, results);
				self.collect(right, variables, results);
			}
			Node::List(items, _, _) => {
				// `for x in xs`: x is an item of xs
				if let [keyword, variable, in_word, list, ..] = items.as_slice() {
					if keyword.name() == FOR_WORD && in_word.name() == IN_WORD {
						if let (Node::Symbol(name), Some(Shape::ListOf(type_name))) = (variable.drop_meta(), self.shape(list)) {
							Self::bind(variables, name, Some(Shape::Instance(type_name)), false);
						}
					}
				}
				items.iter().for_each(|item| self.collect(item, variables, results))
			}
			_ => {}
		}
	}

	/// Is `name` a field of any declared type: `v.x` then reads the field at run time, whatever v holds
	pub fn is_declared_field(&self, name: &str) -> bool {
		self.registry.types().iter().any(|type_def| type_def.fields.iter().any(|field| field.name == name))
	}

	/// The shape of an expression, when it is known to hold instances of one declared type
	pub fn shape(&self, node: &Node) -> Option<Shape> {
		if let Some((name, _)) = instance_parts(node) {
			return Some(Shape::Instance(name.name()));
		}
		if let Some(type_name) = typed_as(node) {
			return Some(Shape::Instance(type_name));
		}
		match node.drop_meta() {
			Node::Symbol(name) => self.variables.get(name).cloned().flatten(),
			Node::List(items, Bracket::Square, _) if !items.is_empty() => {
				let first = self.shape(&items[0])?;
				let Shape::Instance(type_name) = &first else { return None };
				items.iter().all(|item| self.shape(item).as_ref() == Some(&first)).then(|| Shape::ListOf(type_name.clone()))
			}
			// `xs#1`, `xs[0]`: an item of a list of instances
			Node::Key(list, Op::Hash, _) => match self.shape(list)? {
				Shape::ListOf(type_name) => Some(Shape::Instance(type_name)),
				Shape::Instance(_) => None,
			},
			// `xs.sort`
			Node::Key(list, Op::Dot, word) if word.name() == SORT_WORD => self.shape(list).filter(|shape| matches!(shape, Shape::ListOf(_))),
			// `c ? a : b`, `if c then a else b`: the shape both branches share
			Node::Key(_, Op::Question, branches) => match branches.drop_meta() {
				Node::Key(then, Op::Colon, otherwise) => self.branches_shape(then, otherwise),
				_ => None,
			},
			Node::Key(condition_then, Op::Else, otherwise) => match condition_then.drop_meta() {
				Node::Key(_, Op::Then, then) => self.branches_shape(then, otherwise),
				_ => None,
			},
			Node::List(items, _, separator) => match items.as_slice() {
				// `sort xs`, `sort(xs)`
				[word, list] if word.name() == SORT_WORD => self.shape(list).filter(|shape| matches!(shape, Shape::ListOf(_))),
				// `make(t)`: what the user function returns
				[function, ..] if matches!(function.drop_meta(), Node::Symbol(name) if self.results.contains_key(name)) => self.results[&function.name()].clone(),
				// `(x)`, and `(t = a; value)` as min_max binds its operands; another call `f(s)` is not its argument
				[single] => self.shape(single),
				[.., last] if matches!(separator, Separator::Semicolon | Separator::Newline) => self.shape(last),
				_ => None,
			},
			_ => None,
		}
	}

	fn branches_shape(&self, then: &Node, otherwise: &Node) -> Option<Shape> {
		match (diverges(then), diverges(otherwise)) {
			(true, _) => self.shape(otherwise),
			(_, true) => self.shape(then),
			_ => self.shape(then).filter(|shape| self.shape(otherwise).as_ref() == Some(shape)),
		}
	}

	/// A curly list of the declared fields of the instance a node holds: library_words reads fields of it like of an
	/// object literal
	pub fn fields_template(&self, node: &Node) -> Option<Node> {
		let Some(Shape::Instance(type_name)) = self.shape(node) else { return None };
		let type_def = self.registry.get_by_name(&type_name)?;
		let entries = type_def.fields.iter().map(|field| Node::Key(Box::new(Node::Symbol(field.name.clone())), Op::Colon, Box::new(Node::Empty)));
		Some(Node::List(entries.collect(), Bracket::Curly, Separator::Space))
	}
}

/// A branch that never gives a value: the error of `max` over an empty list
fn diverges(branch: &Node) -> bool {
	matches!(branch.drop_meta(), Node::List(items, _, _) if items.first().is_some_and(|head| head.name() == crate::min_max::EMPTY_EXTREMUM_CALL))
}

// ═══════════════════════════════════════════════════════════════════════════
// Static dispatch
// ═══════════════════════════════════════════════════════════════════════════

/// `p < q` of known instances calls their `compare` witness, `p == q` their `equals` override, `area(s)` the `area` of
/// the type of s; ordering, sorting or calling an operation of a type that does not conform is a compile error;
/// `x is shape` of a known instance is answered here
pub fn lower_dispatch(node: Node) -> Node {
	let types = InstanceTypes::of(&node);
	let mut likenesses = vec![];
	collect_declared(&node, &mut likenesses);
	if let Some(error) = undeclared_likeness(&likenesses, &types.registry) {
		return error;
	}
	if types.registry.types().is_empty() {
		return node;
	}
	let witnesses = defined_functions(&node);
	let signatures = instance_signatures(&node, &types.registry);
	let traits = Traits::of(&node);
	let constraints = trait_constraints(&node, &traits);
	let dispatch = Dispatch { traits, types, witnesses, signatures, constraints, likenesses, missing: RefCell::new(None), dispatched: RefCell::new(Default::default()) };
	let node = dispatch.expand(node);
	let node = with_dispatchers(node, &dispatch.dispatched.into_inner());
	dispatch.missing.into_inner().unwrap_or(node)
}

/// `p = field_with(p, "x", v)`: the update of a field of p, which keeps p an instance of its type
fn is_field_update(name: &str, value: &Node) -> bool {
	matches!(value.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(), [word, object, ..]
		if word.name() == crate::library_words::FIELD_WITH && matches!(object.drop_meta(), Node::Symbol(target) if target == name)))
}

/// `instance_of(x, "square")`: 1 when x is an instance of the declared type, else 0 (the emitter reads the type name of
/// the instance at run time)
pub const INSTANCE_OF: &str = "instance_of";
const DISPATCH: &str = "dispatch";

/// The program with a dispatcher per operation called on values of run-time type:
/// `area·dispatch(x) := if instance_of(x, "rect") then area·rect(x) else area·square(x)` (the last type unchecked)
fn with_dispatchers(node: Node, dispatched: &std::collections::BTreeMap<(String, usize), Vec<String>>) -> Node {
	if dispatched.is_empty() {
		return node;
	}
	let mut items: Vec<Node> = dispatched.iter().map(|((operation, arity), types)| dispatcher(operation, *arity, types)).collect();
	items.push(node);
	Node::List(items, Bracket::None, Separator::Semicolon)
}

fn dispatcher(operation: &str, arity: usize, types: &[String]) -> Node {
	let parameters: Vec<Node> = (0..arity).map(|index| Node::Symbol(format!("dispatched_{index}"))).collect();
	let (last, checked) = types.split_last().expect("two or more types define the operation");
	let call = |type_name: &str| witness_call(operation, type_name, parameters.clone());
	let body = checked.iter().rev().fold(call(last), |otherwise, type_name| {
		let test = Node::List(vec![Node::Symbol(INSTANCE_OF.to_string()), parameters[0].clone(), Node::Text(type_name.clone())], Bracket::Round, Separator::None);
		let condition = Node::Key(Box::new(Node::Empty), Op::If, Box::new(test));
		let then = Node::Key(Box::new(condition), Op::Then, Box::new(call(type_name)));
		Node::Key(Box::new(then), Op::Else, Box::new(otherwise))
	});
	// the first parameter is an instance of some type: held as a Node, like a parameter `s:square`
	let instance = Node::Key(Box::new(parameters[0].clone()), Op::Colon, Box::new(Node::meta(Node::Symbol(DISPATCH.to_string()), Node::data(Instance))));
	let head = Node::List([vec![Node::Symbol(witness_name(operation, DISPATCH)), instance], parameters[1..].to_vec()].concat(), Bracket::Round, Separator::None);
	Node::Key(Box::new(head), Op::Define, Box::new(body))
}

struct Dispatch {
	traits: Traits,
	types: InstanceTypes,
	witnesses: Vec<String>,
	/// Per function, its parameters and the declared type each one takes (`p:photo`), if any
	signatures: HashMap<String, Vec<(String, Option<String>)>>,
	/// Per function, the positions of its parameters `xs: Comparable list` and the trait their elements need
	constraints: HashMap<String, Vec<(usize, String)>>,
	/// The `image like photo` declarations
	likenesses: Vec<Likeness>,
	/// The first missing conformance: an error of the whole program, wherever it sits
	missing: RefCell<Option<Node>>,
	/// Operations called on a value whose type is known only at run time: (operation, number of arguments) → the types
	/// that define it, for a dispatcher `op·dispatch` (`dispatchers`)
	dispatched: RefCell<std::collections::BTreeMap<(String, usize), Vec<String>>>,
}

impl Dispatch {
	fn fail(&self, error: Node) -> Option<Node> {
		self.missing.borrow_mut().get_or_insert(error.clone());
		Some(error)
	}

	/// Is `kind` usable as `like`: the same type, or like it through declared likenesses (`thumb like image`, `image like photo`)
	fn is_like(&self, kind: &str, like: &str) -> bool {
		let mut reached = vec![kind.to_string()];
		let mut next = 0;
		while let Some(current) = reached.get(next).cloned() {
			if current == like {
				return true;
			}
			let further = self.likenesses.iter().filter(|likeness| likeness.kind == current).map(|likeness| likeness.like.clone());
			let new: Vec<String> = further.filter(|name| !reached.contains(name)).collect();
			reached.extend(new);
			next += 1;
		}
		false
	}

	fn has_witness(&self, operation: &str, type_name: &str) -> bool {
		self.witnesses.contains(&witness_name(operation, type_name))
	}

	fn instance_type(&self, node: &Node) -> Option<String> {
		match self.types.shape(node)? {
			Shape::Instance(type_name) => Some(type_name),
			Shape::ListOf(_) => None,
		}
	}

	/// The error when the type lacks the built-in operation (`compare` of Comparable), named for what needed it
	fn require(&self, located: &Node, type_name: &str, operation: &str, needed_by: &str) -> Option<Node> {
		let (required, operation) = self.traits.operation(operation)?;
		if self.has_witness(&operation.name, type_name) {
			return None;
		}
		self.fail(missing_conformance(located, type_name, required, operation, needed_by))
	}

	fn expand(&self, node: Node) -> Node {
		match node {
			// a definition head names parameters, it calls nothing
			Node::Key(head, op @ (Op::Assign | Op::Define), body) if definition_head(&head).is_some() => Node::Key(head, op, Box::new(self.expand(*body))),
			Node::Key(left, op, right) => {
				let (left, right) = (self.expand(*left), self.expand(*right));
				if let (Op::Assign, Some((name, declared))) = (&op, typed_target(&left, &self.types.registry)) {
					return match self.admit(&right, &declared, &format!("{name}:{declared} needs {}", crate::analyzer::with_article(&declared))) {
						Ok(right) => Node::Key(Box::new(left), op, Box::new(right)),
						Err(error) => self.fail(error).expect("fail gives the error"),
					};
				}
				self.operator(&left, &op, &right).unwrap_or(Node::Key(Box::new(left), op, Box::new(right)))
			}
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				self.call(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.expand(*node)), data },
			other => other,
		}
	}

	fn operator(&self, left: &Node, op: &Op, right: &Node) -> Option<Node> {
		let type_name = self.instance_type(left).or_else(|| self.instance_type(right))?;
		if op.is_ordering() {
			if let Some(error) = self.require(left, &type_name, COMPARE, &format!("`{op}`")) {
				return Some(error);
			}
			let comparison = witness_call(COMPARE, &type_name, vec![left.clone(), right.clone()]);
			return Some(Node::Key(Box::new(comparison), *op, Box::new(Node::int(0))));
		}
		if matches!(op, Op::Eq | Op::Ne) && self.has_witness(EQUALS, &type_name) {
			let equal = witness_call(EQUALS, &type_name, vec![left.clone(), right.clone()]);
			let wanted = if *op == Op::Eq { Op::Ne } else { Op::Eq };
			return Some(Node::Key(Box::new(equal), wanted, Box::new(Node::int(0))));
		}
		None
	}

	fn call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		if let Some(error) = self.unmet_constraint(items, bracket, separator) {
			return Some(error);
		}
		if let Some(name) = call_name(items, bracket, separator) {
			match (name, &items[1..]) {
				(SORT_WORD, [list]) => {
					let Some(Shape::ListOf(type_name)) = self.types.shape(list) else { return None };
					return self.require(&items[0], &type_name, COMPARE, SORT_WORD);
				}
				// `x in xs` (its 1-based position), `xs.has(x)` of an instance whose type overrides equality: a search by its equals
				(crate::library_words::COLLECTION_POSITION, [list, element]) => {
					let type_name = self.instance_type(element)?;
					return self.has_witness(EQUALS, &type_name).then(|| position_by_equals(list, element, &type_name));
				}
				(crate::library_words::COLLECTION_CONTAINS, [list, element]) => {
					let type_name = self.instance_type(element)?;
					let found = self.has_witness(EQUALS, &type_name).then(|| position_by_equals(list, element, &type_name))?;
					return Some(Node::Key(Box::new(found), Op::Ne, Box::new(Node::int(0))));
				}
				(crate::type_tests::IS_TYPE, [subject, spec]) => {
					let required = self.traits.named(&spec.drop_meta().name())?;
					let type_name = self.instance_type(subject)?;
					let conforms = self.traits.missing(required, &type_name, &self.witnesses).is_none();
					return Some(if conforms { Node::True } else { Node::False });
				}
				_ => {}
			}
		}
		let admitted = match self.admitted_arguments(items, bracket, separator) {
			Ok(admitted) => admitted,
			Err(error) => return self.fail(error),
		};
		match admitted {
			Some(items) => Some(self.operation_call(&items, bracket, separator).unwrap_or(Node::List(items, bracket.clone(), separator.clone()))),
			None => self.operation_call(items, bracket, separator),
		}
	}

	/// `smallest([dot(1)])` of `smallest(xs: Comparable list)`: the error when the elements' type lacks an operation of the trait
	fn unmet_constraint(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let name = called_name(items, bracket, separator)?;
		for (index, trait_name) in self.constraints.get(name)? {
			let Some(argument) = items.get(index + 1) else { continue };
			let Some(Shape::ListOf(type_name)) = self.types.shape(argument) else { continue };
			let required = self.traits.named(trait_name)?;
			if let Some(operation) = self.traits.missing(required, &type_name, &self.witnesses) {
				return self.fail(missing_conformance(&items[0], &type_name, required, operation, &format!("{name}({})", argument.serialize())));
			}
		}
		None
	}

	/// The value given for a place of declared type T (a parameter, a typed variable): the value to use, or the error.
	/// An instance of T or of a type declared like T stays; an instance of another known type is an error that teaches
	/// `like`; written data (`pic{…}` of no declared pic, `{…}`) is used without its ad hoc name, with a warning for each
	/// field it lacks or has extra; a known non-object is an error; anything else is judged by its uses
	fn admit(&self, value: &Node, declared: &str, needs: &str) -> Result<Node, Node> {
		let got = format!("{needs}, got {}", value.serialize());
		if let Some(type_name) = self.instance_type(value) {
			return if self.is_like(&type_name, declared) { Ok(value.clone()) } else { Err(not_like(value, &got, &type_name, declared)) };
		}
		if let Some(data) = written_data(value, &self.types.registry) {
			crate::diagnostic::report(&field_warnings(value, &data, declared, &self.types.registry))?;
			return Ok(data);
		}
		match crate::analyzer::argument_literal_kind(value) {
			Some(kind) => Err(Diagnostic::at(value, format!("{got} ({})", crate::analyzer::kind_with_article(kind))).into_error()),
			None => Ok(value.clone()),
		}
	}

	/// The arguments of a call of a function with class-typed parameters, each admitted for its parameter (`admit`):
	/// None when every argument stays as written
	fn admitted_arguments(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Result<Option<Vec<Node>>, Node> {
		let Some(name) = called_name(items, bracket, separator) else { return Ok(None) };
		let Some(parameters) = self.signatures.get(name) else { return Ok(None) };
		let mut admitted = items.to_vec();
		for (argument, (parameter, declared)) in admitted[1..].iter_mut().zip(parameters) {
			let Some(declared) = declared else { continue };
			*argument = self.admit(argument, declared, &format!("{name} needs {} for parameter {parameter}", crate::analyzer::with_article(declared)))?;
		}
		Ok((admitted != items).then_some(admitted))
	}

	/// `area(s)`, `area s`, `compare(p, q)`: the witness of the type of the first argument
	fn operation_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let [head, first, ..] = items else { return None };
		let name = called_name(items, bracket, separator)?;
		let (required, operation) = self.traits.operation(name)?;
		let arguments = items[1..].to_vec();
		match self.instance_type(first) {
			Some(type_name) if self.has_witness(name, &type_name) => Some(witness_call(name, &type_name, arguments)),
			Some(type_name) => self.fail(missing_conformance(head, &type_name, required, operation, &format!("{name}({})", first.serialize()))),
			// a declared operation of an argument whose type is unknown here: the one type that defines it, else ask for the type
			None if !is_builtin_trait(&required.name) && !self.witnesses.iter().any(|witness| witness == name) => {
				let defining: Vec<&str> = self.witnesses.iter().filter_map(|witness| witness_type(witness, name)).collect();
				match defining.as_slice() {
					[type_name] => Some(witness_call(name, type_name, arguments)),
					// several types define it: the dispatcher picks the witness of the value's type at run time
					[_, _, ..] => {
						let types = defining.iter().map(|type_name| type_name.to_string()).collect();
						self.dispatched.borrow_mut().insert((name.to_string(), arguments.len()), types);
						Some(witness_call(name, DISPATCH, arguments))
					}
					_ => {
						let message = format!("which {name}: the type of {} is not known at compile time ({} define {name})", first.serialize(), defining.join(", "));
						let fix = format!("give it a type, e.g. {}:{}", first.serialize(), defining.first().unwrap_or(&"T"));
						self.fail(Diagnostic::at(head, message).fix(fix).into_error())
					}
				}
			}
			None => None,
		}
	}
}

/// The function a list calls: `f(a, b)`, or spaced `f a b`
fn called_name<'a>(items: &'a [Node], bracket: &Bracket, separator: &Separator) -> Option<&'a str> {
	let Node::Symbol(name) = items.first()?.drop_meta() else { return None };
	let spaced = *separator == Separator::Space && matches!(bracket, Bracket::None | Bracket::Round);
	(call_name(items, bracket, separator).is_some() || spaced).then_some(name.as_str())
}

/// Every function's parameters with the declared type each one takes, for the functions that take any
fn instance_signatures(node: &Node, registry: &TypeRegistry) -> HashMap<String, Vec<(String, Option<String>)>> {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, node);
	let declared = |param: &crate::context::Param| param.annotation.as_ref().map(Node::name).filter(|type_name| registry.get_by_name(type_name).is_some());
	let signature = |function: &crate::context::UserFunctionDef| function.params.iter().map(|param| (param.name.clone(), declared(param))).collect::<Vec<_>>();
	context.user_functions.values().map(|function| (function.name.clone(), signature(function))).filter(|(_, params)| params.iter().any(|(_, declared)| declared.is_some())).collect()
}

/// Every function's parameters annotated `list of <trait>` (`xs: Comparable list`): their positions and traits
fn trait_constraints(node: &Node, traits: &Traits) -> HashMap<String, Vec<(usize, String)>> {
	let mut context = crate::context::Context::new();
	crate::analyzer::extract_user_functions(&mut context, node);
	let constraint = |param: &crate::context::Param| param.annotation.as_ref()?.name().strip_prefix(LIST_OF_PREFIX).filter(|element| traits.is_trait(element)).map(str::to_string);
	context.user_functions.values().map(|function| {
		let constrained = function.params.iter().enumerate().filter_map(|(index, param)| Some((index, constraint(param)?))).collect::<Vec<_>>();
		(function.name.clone(), constrained)
	}).filter(|(_, constrained)| !constrained.is_empty()).collect()
}

/// `(found = 0; at = 0; for item in xs { at = at + 1; if found == 0 and equals·T(item, x) != 0 { found = at } }; found)`:
/// the 1-based position of the first item equal to x, 0 when there is none
fn position_by_equals(list: &Node, element: &Node, type_name: &str) -> Node {
	let symbol = |name: &str| Node::Symbol(name.to_string());
	let (found, at, item) = (symbol(FOUND_VARIABLE), symbol(POSITION_VARIABLE), symbol(ITEM_VARIABLE));
	let assign = |target: &Node, value: Node| Node::Key(Box::new(target.clone()), Op::Assign, Box::new(value));
	let equal = Node::Key(Box::new(witness_call(EQUALS, type_name, vec![item.clone(), element.clone()])), Op::Ne, Box::new(Node::int(0)));
	let first = Node::Key(Box::new(found.clone()), Op::Eq, Box::new(Node::int(0)));
	let condition = Node::Key(Box::new(Node::Empty), Op::If, Box::new(Node::Key(Box::new(first), Op::And, Box::new(equal))));
	let found_it = Node::Key(Box::new(condition), Op::Then, Box::new(Node::List(vec![assign(&found, at.clone())], Bracket::Curly, Separator::None)));
	let step = assign(&at, Node::Key(Box::new(at.clone()), Op::Add, Box::new(Node::int(1))));
	let body = Node::List(vec![step, found_it], Bracket::Curly, Separator::Semicolon);
	let search = Node::List(vec![symbol(FOR_WORD), item, symbol(IN_WORD), list.clone(), body], Bracket::None, Separator::Space);
	Node::List(vec![assign(&found, Node::int(0)), assign(&at, Node::int(0)), search, found], Bracket::Round, Separator::Semicolon)
}
const FOUND_VARIABLE: &str = "equals_found";
const ITEM_VARIABLE: &str = "equals_item";
const POSITION_VARIABLE: &str = "equals_position";

fn witness_call(operation: &str, type_name: &str, arguments: Vec<Node>) -> Node {
	Node::List([vec![Node::Symbol(witness_name(operation, type_name))], arguments].concat(), Bracket::Round, Separator::None)
}
