//! Static dispatch: `p < q` of known instances calls their witness, else a dispatcher per operation; values admitted to places of a declared type
use super::*;
use crate::node::parens;
use crate::words::{ANY_TYPE, INSTANCE_OF};
use crate::Kind;

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
	let traits = Traits::of(&node);
	let signatures = instance_signatures(&node, &types.registry, &traits);
	let enums = crate::lowering::sum_variants::sum_types(&node)
		.into_iter()
		.filter(|(_, variants)| variants.is_plain())
		.map(|(name, _)| name)
		.collect();
	let dispatch = Dispatch {
		traits,
		types,
		witnesses,
		signatures,
		enums,
		likenesses,
		missing: RefCell::new(None),
		dispatched: RefCell::new(Default::default()),
		declared: RefCell::new(vec![HashMap::new()]),
	};
	let node = dispatch.expand(node);
	let node = with_dispatchers(node, &dispatch.dispatched.into_inner());
	dispatch.missing.into_inner().unwrap_or(node)
}

/// `p = field_with(p, "x", v)`: the update of a field of p, which keeps p an instance of its type
pub(super) fn is_field_update(name: &str, value: &Node) -> bool {
	matches!(value.drop_meta(), Node::List(items, _, _) if matches!(items.as_slice(), [word, object, ..]
		if word.name() == crate::library_words::FIELD_WITH && matches!(object.drop_meta(), Node::Symbol(target) if target == name)))
}

pub(super) const DISPATCH: &str = "dispatch";

/// The program with a dispatcher per operation called on values of run-time type:
/// `area·dispatch(x) := if instance_of(x, "rect") then area·rect(x) else area·square(x)` (the last type unchecked)
pub(super) fn with_dispatchers(
	node: Node,
	dispatched: &std::collections::BTreeMap<(String, usize), Vec<String>>,
) -> Node {
	if dispatched.is_empty() {
		return node;
	}
	let mut items: Vec<Node> =
		dispatched.iter().map(|((operation, arity), types)| dispatcher(operation, *arity, types)).collect();
	items.push(node);
	Node::List(items, Bracket::None, Separator::Semicolon)
}

pub(super) fn dispatcher(operation: &str, arity: usize, types: &[String]) -> Node {
	let cases: Vec<DispatchCase> =
		types.iter().map(|type_name| (vec![(0, type_name.clone())], type_name.clone())).collect();
	dispatcher_of_cases(operation, arity, &cases, false)
}

/// `op·dispatch(x, …)`: per case (the classes its arguments are instances of, the method variant that calls) the
/// method; the last case unchecked, or with a `fallback` every case checked and any other value calling `op` itself
pub(super) fn dispatcher_of_cases(operation: &str, arity: usize, cases: &[DispatchCase], fallback: bool) -> Node {
	let parameters: Vec<Node> = (0..arity).map(|index| Node::Symbol(format!("dispatched_{index}"))).collect();
	let method = |variant: &str| witness_call(operation, variant, parameters.clone());
	let (checked, otherwise) = match fallback {
		true => (cases, call(operation, parameters.clone())),
		false => {
			let (last, checked) = cases.split_last().expect("two or more methods");
			(checked, method(&last.1))
		}
	};
	let test = |tests: &[(usize, String)]| {
		tests
			.iter()
			.map(|(position, class)| call(INSTANCE_OF, vec![parameters[*position].clone(), Node::Text(class.clone())]))
			.reduce(|all, test| key(all, Op::And, test))
	};
	let body = checked.iter().rev().fold(otherwise, |otherwise, (tests, variant)| match test(tests) {
		Some(test) => if_then_else(test, method(variant), otherwise),
		None => method(variant),
	});
	// a tested parameter is an instance of some type: held as a Node, like a parameter `s:square`; with a fallback any
	// value, a text too (`sort "cba"` beside a class's sort method)
	let tested: Vec<usize> = cases.iter().flat_map(|(tests, _)| tests.iter().map(|(position, _)| *position)).collect();
	let instance = |position: usize, parameter: &Node| match (tested.contains(&position) || position == 0, fallback) {
		(true, true) => key(parameter.clone(), Op::Colon, symbol(ANY_TYPE)),
		(true, false) => key(parameter.clone(), Op::Colon, Node::meta(symbol(DISPATCH), Node::data(Instance))),
		(false, _) => parameter.clone(),
	};
	let head_parameters = parameters.iter().enumerate().map(|(position, parameter)| instance(position, parameter));
	let head =
		parens(std::iter::once(Node::Symbol(witness_name(operation, DISPATCH))).chain(head_parameters).collect());
	key(head, Op::Define, body)
}

pub(super) struct Dispatch {
	traits: Traits,
	types: InstanceTypes,
	witnesses: Vec<String>,
	/// Per function, its parameters and the declared type each one takes (`p:photo`, `xs: Comparable list`), if any
	signatures: HashMap<String, Vec<(String, Option<String>)>>,
	/// The plain enums, `enum Coin { Penny, Dime }`: an int becomes one of their cases explicitly, `1 as Coin`
	enums: std::collections::HashSet<String>,
	/// The `image like photo` declarations
	likenesses: Vec<Likeness>,
	/// The first missing conformance: an error of the whole program, wherever it sits
	missing: RefCell<Option<Node>>,
	/// Operations called on a value whose type is known only at run time: (operation, number of arguments) → the types
	/// that define it, for a dispatcher `op·dispatch` (`dispatchers`)
	dispatched: RefCell<std::collections::BTreeMap<(String, usize), Vec<String>>>,
	/// The variables declared of a class (`s: C = …`), per definition being rewritten, innermost last: a later `s = v`
	/// is admitted as the declaration was (card class-field)
	declared: RefCell<Vec<HashMap<String, String>>>,
}

impl Dispatch {
	pub(super) fn fail(&self, error: Node) -> Option<Node> {
		self.missing.borrow_mut().get_or_insert(error.clone());
		Some(error)
	}

	/// Is `kind` usable as `like`: the same type, or like it through declared likenesses (`thumb like image`, `image like photo`)
	pub(super) fn is_like(&self, kind: &str, like: &str) -> bool {
		let mut reached = vec![kind.to_string()];
		let mut next = 0;
		while let Some(current) = reached.get(next).cloned() {
			if current == like {
				return true;
			}
			let further = self
				.likenesses
				.iter()
				.filter(|likeness| likeness.kind == current)
				.map(|likeness| likeness.like.clone());
			let new: Vec<String> = further.filter(|name| !reached.contains(name)).collect();
			reached.extend(new);
			next += 1;
		}
		false
	}

	pub(super) fn has_witness(&self, operation: &str, type_name: &str) -> bool {
		self.witnesses.contains(&witness_name(operation, type_name))
	}

	pub(super) fn instance_type(&self, node: &Node) -> Option<String> {
		match self.types.shape(node)? {
			Shape::Instance(type_name) => Some(type_name),
			Shape::ListOf(_) => None,
		}
	}

	/// The error when the type lacks the built-in operation (`compare` of Comparable), named for what needed it
	pub(super) fn require(&self, located: &Node, type_name: &str, operation: &str, needed_by: &str) -> Option<Node> {
		let (required, operation) = self.traits.operation(operation)?;
		if self.has_witness(&operation.name, type_name) {
			return None;
		}
		self.fail(missing_conformance(located, type_name, required, operation, needed_by))
	}

	pub(super) fn expand(&self, node: Node) -> Node {
		match node {
			// a definition head names parameters, it calls nothing
			Node::Key(head, op @ (Op::Assign | Op::Define), body)
				if definition_head(&head).is_some() || is_parameterless_head(&head) =>
			{
				self.declared.borrow_mut().push(HashMap::new());
				let body = self.types.in_definition(&head, || self.expand(*body));
				self.declared.borrow_mut().pop();
				Node::Key(head, op, Box::new(body))
			}
			Node::Key(left, op, right) => {
				let (left, right) = (self.expand(*left), self.expand(*right));
				if let (Op::Assign, Some((place, declared))) = (&op, self.declared_place(&left)) {
					return match self.admit(
						&right,
						&declared,
						&format!("{place}:{declared} needs {}", crate::analyzer::with_article(&declared)),
					) {
						Ok(right) => key(left, op, right),
						Err(error) => self.fail(error).expect("fail gives the error"),
					};
				}
				self.operator(&left, &op, &right).unwrap_or(key(left, op, right))
			}
			// `Box(Point(3))` of `class Box{content: Circle}`: each field given an instance of a declared class admits it
			instance if instance_parts(&instance).is_some() => {
				let instance = children_rewritten(instance, |child| self.expand(child));
				match self.misfit_field(&instance) {
					Some(error) => self.fail(error).expect("fail gives the error"),
					None => instance,
				}
			}
			Node::List(items, bracket, separator) => {
				let items: Vec<Node> = items.into_iter().map(|item| self.expand(item)).collect();
				if let Some(error) = self.misfit_field_write(&items) {
					return self.fail(error).expect("fail gives the error");
				}
				self.call(&items, &bracket, &separator).unwrap_or(Node::List(items, bracket, separator))
			}
			other => children_rewritten(other, |child| self.expand(child)),
		}
	}

	pub(super) fn operator(&self, left: &Node, op: &Op, right: &Node) -> Option<Node> {
		let type_name = self.instance_type(left).or_else(|| self.instance_type(right))?;
		if op.is_ordering() {
			if let Some(error) = self.require(left, &type_name, COMPARE, &format!("`{op}`")) {
				return Some(error);
			}
			let comparison = witness_call(COMPARE, &type_name, vec![left.clone(), right.clone()]);
			return Some(key(comparison, *op, Node::int(0)));
		}
		if matches!(op, Op::Eq | Op::Ne) && self.has_witness(EQUALS, &type_name) {
			let equal = witness_call(EQUALS, &type_name, vec![left.clone(), right.clone()]);
			let wanted = if *op == Op::Eq { Op::Ne } else { Op::Eq };
			return Some(key(equal, wanted, Node::int(0)));
		}
		None
	}

	pub(super) fn call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		if let Some(name) = call_name(items, bracket, separator) {
			match (name, &items[1..]) {
				(SORT_WORD, [list]) => {
					let Some(Shape::ListOf(type_name)) = self.types.shape(list) else {
						return None;
					};
					return self.require(&items[0], &type_name, COMPARE, SORT_WORD);
				}
				// `x in xs` (its 1-based position), `xs.has(x)` of an instance whose type overrides equality: a search by its equals
				(crate::library_words::COLLECTION_POSITION, [list, element]) => {
					let type_name = self.instance_type(element)?;
					return self.has_witness(EQUALS, &type_name).then(|| position_by_equals(list, element, &type_name));
				}
				(crate::library_words::COLLECTION_CONTAINS, [list, element]) => {
					let type_name = self.instance_type(element)?;
					let found =
						self.has_witness(EQUALS, &type_name).then(|| position_by_equals(list, element, &type_name))?;
					return Some(key(found, Op::Ne, Node::int(0)));
				}
				// `x is Comparable` of a trait, `dog(…) is animal` of a type dog is like (class dog extends animal)
				(crate::words::IS_TYPE, [subject, spec]) => {
					let spec = spec.drop_meta().name();
					let type_name = self.instance_type(subject)?;
					let conforms = match self.traits.named(&spec) {
						Some(required) => self.traits.missing(required, &type_name, &self.witnesses).is_none(),
						None if type_name != spec && self.is_like(&type_name, &spec) => true,
						None => return None,
					};
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
			Some(items) => Some(self.operation_call(&items, bracket, separator).unwrap_or(Node::List(
				items,
				bracket.clone(),
				separator.clone(),
			))),
			None => self.operation_call(items, bracket, separator),
		}
	}

	/// The value given for a place of declared type T (a parameter, a typed variable): the value to use, or the error.
	/// An instance of T or of a type declared like T stays; an instance of another known type is an error that teaches
	/// `like`; written data (`pic{…}` of no declared pic, `{…}`) is used without its ad hoc name, with a warning for each
	/// field it lacks or has extra; a known non-object is an error; anything else is judged by its uses
	pub(super) fn admit(&self, value: &Node, declared: &str, needs: &str) -> Result<Node, Node> {
		// `smallest([dot(1)])` of `smallest(xs: Comparable list)`: each operation of the trait needs its witness
		if let Some(required) = self.traits.element_trait(declared) {
			return match self.types.shape(value) {
				Some(Shape::ListOf(type_name))
					if let Some(operation) = self.traits.missing(required, &type_name, &self.witnesses) =>
				{
					Err(missing_conformance(value, &type_name, required, operation, &value.serialize()))
				}
				_ => Ok(value.clone()),
			};
		}
		let got = format!("{needs}, got {}", value.serialize());
		let parts = class_parts(declared);
		if let Some(type_name) = self.instance_type(value) {
			let classes: Vec<&str> =
				parts.iter().copied().filter(|part| self.types.registry.get_by_name(part).is_some()).collect();
			return match parts.as_slice() {
				_ if classes.iter().any(|class| self.is_like(&type_name, class)) => Ok(value.clone()),
				[_] => Err(not_like(value, &got, &type_name, declared)),
				_ => Err(Diagnostic::at(value, format!("{got}: {type_name} is none of them"))
					.fix(format!("declare `{type_name} like {}` to use it as one", classes[0]))
					.into_error()),
			};
		}
		if let Some(data) = written_data(value, &self.types.registry) {
			crate::errors::diagnostic::report(&field_warnings(value, &data, declared, &self.types.registry))?;
			return Ok(data);
		}
		// `p: Point | text` takes a text too
		match crate::analyzer::argument_literal_kind(value).filter(|kind| {
			!parts
				.iter()
				.any(|part| crate::analyzer::builtin_type_kind(part).is_some() && crate::analyzer::admits(part, *kind))
		}) {
			Some(kind) => {
				let error = Diagnostic::at(value, format!("{got} ({})", crate::analyzer::kind_with_article(kind)));
				Err(match kind == Kind::Int && self.enums.contains(declared) {
					// decision enum-case B: the int is the index of a case, said explicitly
					true => {
						let written = value.serialize();
						let case = format!("{written} as {declared}");
						error.fix(format!("write {case}")).offer(
							format!("the {declared} case at index {written}"),
							written,
							case,
						)
					}
					false => error,
				}
				.into_error())
			}
			None => Ok(value.clone()),
		}
	}

	/// The arguments of a call of a function with class-typed parameters, each admitted for its parameter (`admit`):
	/// None when every argument stays as written
	pub(super) fn admitted_arguments(
		&self,
		items: &[Node],
		bracket: &Bracket,
		separator: &Separator,
	) -> Result<Option<Vec<Node>>, Node> {
		let Some(name) = called_name(items, bracket, separator) else {
			return Ok(None);
		};
		let Some(parameters) = self.signatures.get(name) else {
			return Ok(None);
		};
		let mut admitted = items.to_vec();
		for (argument, (parameter, declared)) in admitted[1..].iter_mut().zip(parameters) {
			let Some(declared) = declared else { continue };
			*argument = self.admit(
				argument,
				declared,
				&format!("{name} needs {} for parameter {parameter}", crate::analyzer::with_article(declared)),
			)?;
		}
		Ok((admitted != items).then_some(admitted))
	}

	/// The variable an assignment writes and the class it is declared of: `s: C = …` (remembered), a later `s = …` of it
	pub(super) fn declared_place(&self, target: &Node) -> Option<(String, String)> {
		if let Some((name, declared)) = typed_target(target, &self.types.registry) {
			self.declared.borrow_mut().last_mut().expect("main's scope").insert(name.clone(), declared.clone());
			return Some((name, declared));
		}
		let Node::Symbol(name) = target.drop_meta() else {
			return None;
		};
		self.declared.borrow().last()?.get(name).map(|declared| (name.clone(), declared.clone()))
	}

	/// `b.content = v` is `b = field_with(b, "content", v)` here: the error of a value the field's declared class does not take
	pub(super) fn misfit_field_write(&self, items: &[Node]) -> Option<Node> {
		let [head, object, field, value] = items else {
			return None;
		};
		if head.symbol_name() != Some(crate::library_words::FIELD_WITH) {
			return None;
		}
		let Node::Text(field) = field.drop_meta() else {
			return None;
		};
		let class = self.instance_type(object)?;
		let declared = self.class_field_type(&class, field, value)?;
		self.admit(
			value,
			&declared,
			&format!("{class}.{field}:{declared} needs {}", crate::analyzer::with_article(&declared)),
		)
		.err()
	}

	/// The declared type of a field, when it names classes (`content: Circle`, `content: Circle | Square`); a field of a
	/// class may also hold ø, as `left: Node` of a tree does at its leaves
	pub(super) fn class_field_type(&self, class: &str, field: &str, value: &Node) -> Option<String> {
		if value.is_empty_node() {
			return None;
		}
		let type_def = self.types.registry.get_by_name(class)?;
		let declared = type_def.fields.iter().find(|declared| declared.name == field)?.type_name.trim_end_matches('?');
		names_classes(declared, &self.types.registry).then(|| declared.to_string())
	}

	/// The error of the first field of an instance given a value its declared class type does not take
	pub(super) fn misfit_field(&self, instance: &Node) -> Option<Node> {
		let (name, fields) = instance_parts(instance)?;
		let class = name.drop_meta().name();
		let Node::List(entries, _, _) = fields.drop_meta() else {
			return None;
		};
		entries.iter().find_map(|entry| {
			let Node::Key(field, Op::Colon, value) = entry.drop_meta() else {
				return None;
			};
			let field = field.drop_meta().name();
			let declared = self.class_field_type(&class, &field, value)?;
			self.admit(value, &declared, &format!("{class}.{field} is {}", crate::analyzer::with_article(&declared)))
				.err()
		})
	}

	/// `area(s)`, `area s`, `compare(p, q)`: the witness of the type of the first argument
	pub(super) fn operation_call(&self, items: &[Node], bracket: &Bracket, separator: &Separator) -> Option<Node> {
		let [head, first, ..] = items else { return None };
		let name = called_name(items, bracket, separator)?;
		let (required, operation) = self.traits.operation(name)?;
		let arguments = items[1..].to_vec();
		match self.instance_type(first) {
			Some(type_name) if self.has_witness(name, &type_name) => Some(witness_call(name, &type_name, arguments)),
			Some(type_name) => self.fail(missing_conformance(
				head,
				&type_name,
				required,
				operation,
				&format!("{name}({})", first.serialize()),
			)),
			// a declared operation of an argument whose type is unknown here: the one type that defines it, else ask for the type
			None if !is_builtin_trait(&required.name) && !self.witnesses.iter().any(|witness| witness == name) => {
				let defining: Vec<&str> =
					self.witnesses.iter().filter_map(|witness| witness_type(witness, name)).collect();
				match defining.as_slice() {
					[type_name] => Some(witness_call(name, type_name, arguments)),
					// several types define it: the dispatcher picks the witness of the value's type at run time
					[_, _, ..] => {
						let types = defining.iter().map(|type_name| type_name.to_string()).collect();
						self.dispatched.borrow_mut().insert((name.to_string(), arguments.len()), types);
						Some(witness_call(name, DISPATCH, arguments))
					}
					_ => {
						let message = format!(
							"which {name}: the type of {} is not known at compile time ({} define {name})",
							first.serialize(),
							defining.join(", ")
						);
						let fix =
							format!("give it a type, e.g. {}:{}", first.serialize(), defining.first().unwrap_or(&"T"));
						self.fail(Diagnostic::at(head, message).fix(fix).into_error())
					}
				}
			}
			None => None,
		}
	}
}
