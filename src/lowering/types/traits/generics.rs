//! Generic functions: methods told apart by the classes of their arguments (`collide(a:Ship, b:Rock)`), each call resolved or dispatched
use super::*;
use crate::words::ANY_TYPE;
use std::collections::HashSet;

/// The class each positional parameter of a method takes, None where it takes any value: `collide(a:Ship, b:Rock)`
pub(super) type Signature = Vec<Option<String>>;
/// A method: its signature and the numbers of arguments it takes, fewest and most
pub(super) type Method = (Signature, (usize, usize));

/// How far an argument fits a parameter that takes any value: further than any class it is like
pub(super) const ANY_DISTANCE: usize = usize::MAX;

/// A function name with methods for declared classes (`def sort(d:Deck)`, `collide(a:Ship, b:Rock)`, notes/dispatch.md
/// "Methods"): a definition adds a method, it never replaces the function for other values
pub(crate) struct Generic {
	/// The methods in declaration order: their signatures and the numbers of arguments they take
	methods: Vec<Method>,
	/// The parameter names of the first method, for the fix of a missing method
	parameters: Vec<String>,
	/// Other values call the library word: `sort` of lists
	library: bool,
	/// The untyped definitions other values call, by the arguments they take: `sort(x)` (1, 1), `take(xs, n)` (2, 2)
	untyped_arities: Vec<(usize, usize)>,
}

/// The method a call picks: its index, none fitting, or two fitting equally well
pub(super) type Picked = Result<Option<usize>, (usize, usize)>;

impl Generic {
	/// The method for arguments of these classes (None: no instance): of the methods taking them, the one at least as
	/// close as every other at each position (a class's own method before its parent's, a class before any value);
	/// two methods each closer somewhere are ambiguous
	pub(super) fn method_for(&self, classes: &[Option<String>], likenesses: &[Likeness]) -> Picked {
		let fitting: Vec<(usize, Vec<usize>)> = self
			.methods
			.iter()
			.enumerate()
			.filter(|(_, (_, (required, all)))| (*required..=*all).contains(&classes.len()))
			.filter_map(|(index, (signature, _))| Some((index, distances(signature, classes, likenesses)?)))
			.collect();
		let dominates =
			|closer: &[usize], further: &[usize]| closer.iter().zip(further).all(|(a, b)| a <= b) && closer != further;
		let best: Vec<usize> = fitting
			.iter()
			.filter(|(_, own)| !fitting.iter().any(|(_, other)| dominates(other, own)))
			.map(|(index, _)| *index)
			.collect();
		match best.as_slice() {
			[] => Ok(None),
			[one] => Ok(Some(*one)),
			[first, second, ..] => Err((*first, *second)),
		}
	}

	/// The positions some method takes a class at: those a dispatcher tests
	pub(super) fn typed_positions(&self) -> Vec<usize> {
		let mut positions: Vec<usize> = self
			.methods
			.iter()
			.flat_map(|(signature, _)| {
				signature.iter().enumerate().filter(|(_, class)| class.is_some()).map(|(position, _)| position)
			})
			.collect();
		positions.sort();
		positions.dedup();
		positions
	}

	/// Whether other values of a call with `arity` arguments have a function to call: `b.take()` of a class Box
	/// never calls list's `take(xs, n)`
	pub(super) fn falls_back(&self, arity: usize) -> bool {
		self.library || self.untyped_arities.iter().any(|(required, all)| (*required..=*all).contains(&arity))
	}

	pub(super) fn variant(&self, method: usize) -> String {
		signature_variant(&self.methods[method].0)
	}

	pub(super) fn signatures(&self, name: &str) -> String {
		self.methods
			.iter()
			.map(|(signature, _)| format!("{name}({})", classes_text(signature)))
			.collect::<Vec<_>>()
			.join(", ")
	}
}

/// Per position, how many likeness steps lead from the argument's class to the parameter's (0: its own class); None
/// where the method does not take the argument
pub(super) fn distances(
	signature: &Signature,
	classes: &[Option<String>],
	likenesses: &[Likeness],
) -> Option<Vec<usize>> {
	classes
		.iter()
		.enumerate()
		.map(|(position, class)| match signature.get(position).cloned().flatten() {
			None => Some(ANY_DISTANCE),
			Some(parameter) => likeness_steps(class.as_deref()?, &parameter, likenesses),
		})
		.collect()
}

/// The likeness steps from `class` to `target`: a dog is 1 step from an animal it is like
pub(super) fn likeness_steps(class: &str, target: &str, likenesses: &[Likeness]) -> Option<usize> {
	let (mut level, mut seen) = (vec![class.to_string()], vec![class.to_string()]);
	for steps in 0.. {
		if level.is_empty() {
			return None;
		}
		if level.iter().any(|kind| kind == target) {
			return Some(steps);
		}
		let mut parents: Vec<String> = level
			.iter()
			.flat_map(|kind| {
				likenesses.iter().filter(move |likeness| likeness.kind == *kind).map(|likeness| likeness.like.clone())
			})
			.collect();
		parents.retain(|parent| !seen.contains(parent));
		parents.dedup();
		seen.extend(parents.iter().cloned());
		level = parents;
	}
	None
}

/// `Ship·Rock` of `collide(a:Ship, b:Rock)`, `Deck` of `sort(d:Deck)`, `any·Ship` of `describe(k, s:Ship)`
pub(super) fn signature_variant(signature: &Signature) -> String {
	let last_class = signature.iter().rposition(Option::is_some).map_or(0, |position| position + 1);
	signature[..last_class]
		.iter()
		.map(|class| class.as_deref().unwrap_or(ANY_TYPE))
		.collect::<Vec<_>>()
		.join(&WITNESS_SEPARATOR.to_string())
}

/// `Ship, Rock`; `Deck` alone
pub(super) fn classes_text(classes: &[Option<String>]) -> String {
	classes.iter().map(|class| class.as_deref().unwrap_or(ANY_TYPE)).collect::<Vec<_>>().join(", ")
}

/// The class a parameter `s:square` takes, if it is a declared one, per positional parameter of a definition
pub(super) fn signature_of(parameters: &[Node], registry: &TypeRegistry) -> Signature {
	parameters.iter().map(|parameter| parameter_type(parameter, registry).map(String::from)).collect()
}

/// The functions with methods for declared classes that need telling apart: methods of several signatures, or of a
/// name with a fallback. A declared trait's operations stay the traits' (witness_of)
pub(super) fn generic_functions(node: &Node, registry: &TypeRegistry, traits: &Traits) -> HashMap<String, Generic> {
	let mut typed: Vec<(String, Method, Vec<String>)> = vec![];
	let mut untyped: Vec<(String, (usize, usize))> = vec![];
	node.visit(&mut |part| {
		if let Node::Key(head, Op::Assign | Op::Define, _) = part {
			let Some(items) = definition_head(head) else { return };
			let Node::Symbol(name) = items[0].drop_meta() else {
				return;
			};
			let witness = name.contains(WITNESS_SEPARATOR) && !crate::lowering::overloads::is_arity_variant(name);
			if witness || traits.operation(name).is_some() {
				return;
			}
			let signature = signature_of(&items[1..], registry);
			match signature.iter().any(Option::is_some) {
				true => typed.push((
					name.clone(),
					(signature, arities(&items[1..])),
					items[1..].iter().map(|parameter| parameter.drop_meta().name()).collect(),
				)),
				false => untyped.push((name.clone(), arities(&items[1..]))),
			}
		}
	});
	let mut generics: HashMap<String, Generic> = HashMap::new();
	for (name, (signature, arity), parameters) in typed {
		let library = crate::lowering::class_methods::is_library_method(&name);
		let untyped_arities =
			untyped.iter().filter(|(untyped_name, _)| *untyped_name == name).map(|(_, arity)| *arity).collect();
		let generic = generics.entry(name).or_insert(Generic { methods: vec![], parameters, library, untyped_arities });
		if !generic.methods.iter().any(|(known, _)| *known == signature) {
			generic.methods.push((signature, arity));
		}
	}
	generics.retain(|_, generic| generic.library || !generic.untyped_arities.is_empty() || generic.methods.len() > 1);
	generics
}

/// The arguments a definition takes, fewest and most: `add(a, b = 1)` takes 1 or 2
pub(super) fn arities(parameters: &[Node]) -> (usize, usize) {
	let required =
		parameters.iter().filter(|parameter| !matches!(parameter.drop_meta(), Node::Key(_, Op::Assign, _))).count();
	(required, parameters.len())
}

/// `collide(a:Ship, b:Rock)` of a generic function: the method `collide·Ship·Rock`
pub(super) fn method_of(head: &Node, registry: &TypeRegistry, generics: &HashMap<String, Generic>) -> Option<String> {
	let items = definition_head(head)?;
	let name = items[0].symbol_name()?;
	generics.contains_key(name).then_some(())?;
	let signature = signature_of(&items[1..], registry);
	signature.iter().any(Option::is_some).then(|| witness_name(name, &signature_variant(&signature)))
}

/// Every call of a generic function calls the method for its arguments' classes: statically where they are known,
/// through a dispatcher `f·dispatch` where they are known only at run time; other values call the fallback
pub(super) fn generic_calls(node: Node, functions: HashMap<String, Generic>) -> Result<Node, Node> {
	let types = InstanceTypes::of(&node);
	let mut likenesses = vec![];
	collect_declared(&node, &mut likenesses);
	let plain = crate::analyzer::literal_variable_kinds(&node, &crate::analyzer::function_context(&node));
	let classes = types.registry.types().iter().map(|type_def| type_def.name.clone()).collect();
	let path_variables = crate::warp_parser::path_variables(&node);
	let calls = GenericCalls {
		path_variables,
		functions,
		types,
		likenesses,
		plain,
		classes,
		failed: RefCell::new(None),
		dispatched: RefCell::new(Default::default()),
	};
	let node = calls.resolve(node);
	if let Some(error) = calls.failed.into_inner() {
		return Err(error);
	}
	let dispatchers: Vec<Node> = calls
		.dispatched
		.into_inner()
		.into_iter()
		.map(|((name, arity), (cases, fallback))| dispatcher_of_cases(&name, arity, &cases, fallback))
		.collect();
	Ok(match dispatchers.is_empty() {
		true => node,
		false => Node::List([dispatchers, vec![node]].concat(), Bracket::None, Separator::Semicolon),
	})
}

pub(super) struct GenericCalls {
	functions: HashMap<String, Generic>,
	types: InstanceTypes,
	likenesses: Vec<Likeness>,
	/// The variables only ever holding literals, which are no instances
	plain: HashMap<String, crate::type_kinds::Kind>,
	classes: Vec<String>,
	/// The variables only ever holding paths written without quotes (`notes = ~/notes.txt`)
	path_variables: HashSet<String>,
	failed: RefCell<Option<Node>>,
	/// (function, number of arguments) → its dispatcher's cases
	dispatched: RefCell<std::collections::BTreeMap<(String, usize), DispatchCases>>,
}

/// A dispatcher's case: the classes its arguments are instances of (position, class), and the method variant it calls
pub(super) type DispatchCase = (Vec<(usize, String)>, String);
/// A dispatcher's cases and whether other values call the fallback
pub(super) type DispatchCases = (Vec<DispatchCase>, bool);

/// What a call knows of an argument: its class, no instance at all (None), or nothing until run time
pub(super) enum Known {
	Class(Option<String>),
	AtRunTime,
}

impl GenericCalls {
	pub(super) fn fail(&self, error: Node) -> Option<Node> {
		self.failed.borrow_mut().get_or_insert(error.clone());
		Some(error)
	}

	pub(super) fn resolve(&self, node: Node) -> Node {
		match node {
			Node::Key(head, op @ (Op::Assign | Op::Define), body) if definition_head(&head).is_some() => {
				let body = self.types.in_definition(&head, || self.resolve(*body));
				Node::Key(head, op, Box::new(body))
			}
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.resolve(item)).collect();
				self.call(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			other => children_rewritten(other, |child| self.resolve(child)),
		}
	}

	/// The kind of a value certainly no instance: a literal, a variable only ever holding literals, a list of instances
	pub(super) fn plain_kind(&self, value: &Node) -> Option<crate::type_kinds::Kind> {
		let literal = crate::analyzer::argument_literal_kind(value).or_else(|| match value.drop_meta() {
			Node::Symbol(name) if !self.types.is_parameter(name) => self.plain.get(name).copied(),
			_ => None,
		});
		literal.or_else(|| {
			matches!(self.types.shape(value), Some(Shape::ListOf(_))).then_some(crate::type_kinds::Kind::List)
		})
	}

	pub(super) fn known(&self, argument: &Node) -> Known {
		match (self.types.shape(argument), self.plain_kind(argument)) {
			(Some(Shape::Instance(class)), _) => Known::Class(Some(class)),
			(_, Some(_)) => Known::Class(None),
			_ => Known::AtRunTime,
		}
	}

	pub(super) fn call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let [head, _, ..] = items else { return None };
		let name = called_name(items, bracket, separator)?;
		let generic = self.functions.get(name)?;
		let arguments: Vec<Node> = items[1..]
			.iter()
			.enumerate()
			.map(|(position, argument)| self.as_path(generic, position, argument))
			.collect();
		let fallback = generic.falls_back(arguments.len());
		let known: Vec<Known> = arguments.iter().map(|argument| self.known(argument)).collect();
		let positions = generic.typed_positions();
		let is_known = |position: &usize| known.get(*position).is_none_or(|known| matches!(known, Known::Class(_)));
		if positions.iter().all(is_known) {
			let classes: Vec<Option<String>> = known
				.iter()
				.map(|known| match known {
					Known::Class(class) => class.clone(),
					Known::AtRunTime => None,
				})
				.collect();
			return match generic.method_for(&classes, &self.likenesses) {
				Ok(Some(method)) => Some(witness_call(name, &generic.variant(method), arguments)),
				Ok(None) if fallback => None,
				Ok(None) => self.fail(no_method(head, name, &classes, generic)),
				Err((first, second)) => self.fail(ambiguous_method(head, name, &classes, generic, first, second)),
			};
		}
		let cases = self.cases(generic, &positions, arguments.len());
		let variants: std::collections::HashSet<&String> = cases.iter().map(|(_, variant)| variant).collect();
		match (variants.len(), fallback) {
			(0, true) => None,
			(1, false) => Some(witness_call(name, &cases[0].1, arguments)),
			_ => {
				self.dispatched.borrow_mut().insert((name.to_string(), arguments.len()), (cases, fallback));
				Some(witness_call(name, DISPATCH, arguments))
			}
		}
	}

	/// `sort ./names.txt` with a method sort(file:Path) (lib/shell.warp): a path written without quotes, or a variable
	/// only holding such paths, becomes a Path where a method takes one; elsewhere it stays the text
	fn as_path(&self, generic: &Generic, position: usize, argument: &Node) -> Node {
		let written_as_path = crate::warp_parser::is_written_path(argument, &self.path_variables);
		let takes_path = generic.methods.iter().any(|(signature, _)| {
			signature.get(position).is_some_and(|class| class.as_deref() == Some(crate::words::PATH_CLASS))
		});
		match written_as_path && takes_path {
			true => {
				let name = key(symbol(crate::words::PATH_FIELD), Op::Colon, without_path_mark(argument));
				crate::type_constructor::instance_node(crate::words::PATH_CLASS, vec![name])
			}
			false => argument.clone(),
		}
	}

	/// Per combination of classes at the typed positions (or no instance there), the method it calls: the combinations
	/// testing more classes first, so a value of no class reaches only methods taking any value there
	pub(super) fn cases(&self, generic: &Generic, positions: &[usize], arity: usize) -> Vec<DispatchCase> {
		let positions: Vec<usize> = positions.iter().copied().filter(|position| *position < arity).collect();
		let candidates = |position: usize| -> Vec<Option<String>> {
			let parameters: Vec<&String> =
				generic.methods.iter().filter_map(|(signature, _)| signature.get(position)?.as_ref()).collect();
			let fitting = self.classes.iter().filter(|class| {
				parameters.iter().any(|parameter| likeness_steps(class, parameter, &self.likenesses).is_some())
			});
			fitting.cloned().map(Some).chain([None]).collect()
		};
		let mut combinations: Vec<Vec<(usize, Option<String>)>> = vec![vec![]];
		for position in &positions {
			combinations = combinations
				.into_iter()
				.flat_map(|combination| {
					candidates(*position)
						.into_iter()
						.map(move |class| [combination.clone(), vec![(*position, class)]].concat())
				})
				.collect();
		}
		let mut cases: Vec<DispatchCase> = combinations
			.into_iter()
			.filter_map(|combination| {
				let mut classes: Vec<Option<String>> = vec![None; arity];
				combination.iter().for_each(|(position, class)| classes[*position] = class.clone());
				let method = generic.method_for(&classes, &self.likenesses).ok()??;
				let tests = combination.into_iter().filter_map(|(position, class)| Some((position, class?))).collect();
				Some((tests, generic.variant(method)))
			})
			.collect();
		cases.sort_by_key(|(tests, _)| std::cmp::Reverse(tests.len()));
		cases
	}
}

/// `area(circle(1))` of a generic function without a method for circle
pub(super) fn no_method(located: &Node, name: &str, classes: &[Option<String>], generic: &Generic) -> Node {
	let typed: Vec<&Option<String>> = classes.iter().filter(|class| class.is_some()).collect();
	let described = match typed.as_slice() {
		[class] if classes.len() == 1 || generic.typed_positions() == [0] => {
			class.as_deref().unwrap_or_default().to_string()
		}
		_ => format!("({})", classes_text(classes)),
	};
	let parameters: Vec<String> = generic
		.parameters
		.iter()
		.zip(classes)
		.map(|(parameter, class)| match class {
			Some(class) => format!("{parameter}:{class}"),
			None => parameter.clone(),
		})
		.collect();
	let message = format!("{name} has no method for {described}; methods: {}", generic.signatures(name));
	Diagnostic::at(located, message).fix(format!("define {name}({}) := …", parameters.join(", "))).into_error()
}

/// `f(C(1))` of methods f(A) and f(B) where C is like both; `meet(dog, dog)` of meet(dog, animal) and meet(animal, dog)
pub(super) fn ambiguous_method(
	located: &Node,
	name: &str,
	classes: &[Option<String>],
	generic: &Generic,
	first: usize,
	second: usize,
) -> Node {
	let (first, second) = (&generic.methods[first].0, &generic.methods[second].0);
	let message = match (classes, first.as_slice(), second.as_slice()) {
		([Some(class)], [Some(a)], [Some(b)]) => format!(
			"{name} of {} is ambiguous: {class} is like both {a} and {b}, which have a method {name} each",
			crate::analyzer::with_article(class)
		),
		_ => format!(
			"{name}({}) is ambiguous: {name}({}) and {name}({}) fit equally well",
			classes_text(classes),
			classes_text(first),
			classes_text(second)
		),
	};
	let parameters: Vec<String> = classes
		.iter()
		.enumerate()
		.map(|(position, class)| match class {
			Some(class) => {
				format!("{}:{class}", generic.parameters.get(position).map_or(DEFAULT_PARAMETER, String::as_str))
			}
			None => generic.parameters.get(position).cloned().unwrap_or_default(),
		})
		.collect();
	Diagnostic::at(located, message)
		.fix(format!("define {name}({}) := … to pick one", parameters.join(", ")))
		.into_error()
}

fn without_path_mark(argument: &Node) -> Node {
	match argument {
		Node::Meta { node, .. } if crate::warp_parser::is_path_literal(argument) => without_path_mark(node),
		other => other.clone(),
	}
}
