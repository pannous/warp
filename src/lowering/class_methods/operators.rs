//! Operators on instances (wiki/operator.md): `a + b` calls the class's `plus`, quantities with run-time units, and
//! functions specialized by the classes of their arguments
use super::*;


/// `a + b` of an instance a whose class defines the operator's method (`plus`, Python's `__add__`, wiki/operator.md):
/// the method call `a.plus(b)`; an operation of such calls is an instance of the same class (`a + b + c`)
pub(super) fn operator_calls(node: Node) -> Node {
	let mut methods: Vec<(String, Op, String)> = vec![];
	node.visit(&mut |part| if let Node::Type { name, body } = part {
		for (method, _, _) in class_items(body).iter().filter_map(method_parts) {
			if let Some((op, names)) = OPERATOR_METHODS.iter().find(|(_, names)| names.contains(&method.as_str())) {
				if method != names[0] {
					crate::normalize::set_position_of(part);
					crate::diagnostic::note_alias(&method, names[0]);
				}
				methods.push((name.drop_meta().name(), *op, method));
			}
		}
	});
	if methods.is_empty() {
		return node;
	}
	let defines = |methods: &[(String, Op, String)], class: &str, op: Op| methods.iter().find(|(owner, known, _)| owner == class && *known == op).map(|(_, _, method)| method.clone());
	for (class, _, _) in methods.clone() {
		for (defined, missing) in INTERCHANGEABLE_OPERATORS {
			if let (Some(method), None) = (defines(&methods, &class, defined), defines(&methods, &class, missing)) {
				methods.push((class.clone(), missing, method));
			}
		}
	}
	let classes: Vec<String> = methods.iter().map(|(class, _, _)| class.clone()).collect();
	let instances = instance_classes(&node, &classes);
	let returned = returned_classes(&node, &classes);
	let no_parameters = std::collections::HashMap::new();
	let operands = Operands { methods: &methods, instances: &instances, returned: &returned, parameters: &no_parameters };
	let node = specialized_calls(node, &operands);
	let parameters = parameter_classes(&argument_classes(&node, &operands));
	with_operator_calls(node, &Operands { parameters: &parameters, ..operands }).0
}

/// Each class's fields: (name, type) in declaration order
pub(crate) type ClassFields = std::collections::HashMap<String, Vec<(String, String)>>;
/// The class of a value, when known
pub(super) type ClassOf<'a> = dyn Fn(&Node) -> Option<String> + 'a;
/// Per defined function, the operator class each argument of each of its calls gives (None: a plain value or unknown)
pub(super) type CallClasses = std::collections::HashMap<String, Vec<Vec<Option<String>>>>;

pub(super) fn argument_classes(node: &Node, operands: &Operands) -> CallClasses {
	let defined: std::collections::HashSet<String> = definition_heads(node).into_iter().map(|items| items[0].drop_meta().name()).collect();
	let mut calls = CallClasses::new();
	collect_argument_classes(node, operands, &defined, &mut calls);
	calls
}

/// The class of each parameter that every call of its function gives an instance of: `f(x) := x * 2; f(5 m ± 1 cm)`
/// (card plus-minus-units); None at a position where the calls differ or give no instance
pub(super) fn parameter_classes(calls: &CallClasses) -> std::collections::HashMap<String, Vec<Option<String>>> {
	let agreed = |calls: &Vec<Vec<Option<String>>>| calls[1..].iter().fold(calls[0].clone(), |known, given| {
		known.iter().zip(given).map(|(known, given)| known.clone().filter(|known| Some(known) == given.as_ref())).collect()
	});
	calls.iter().map(|(function, calls)| (function.clone(), agreed(calls))).collect()
}

/// `f(x) := x * 2` called with a quantity and with a plain number (card mixed-arguments): each call whose argument is
/// an instance where other calls give something else calls a copy typed by that class, `f_Quantity(x:Quantity)`,
/// so the operators in it are the class's methods and the plain calls keep f. Only the program's own functions with
/// untyped parameters are copied, not methods
pub(super) fn specialized_calls(node: Node, operands: &Operands) -> Node {
	let calls = argument_classes(&node, operands);
	let copyable = copyable_functions(&node);
	let mixed: std::collections::HashMap<String, Vec<bool>> = calls.iter().filter(|(function, _)| copyable.contains(*function)).filter_map(|(function, calls)| {
		let positions = (0..calls[0].len()).map(|position| {
			let given: Vec<&Option<String>> = calls.iter().filter_map(|call| call.get(position)).collect();
			given.iter().any(|class| class.is_some()) && given.iter().any(|class| *class != given[0])
		}).collect::<Vec<bool>>();
		positions.contains(&true).then(|| (function.clone(), positions))
	}).collect();
	if mixed.is_empty() {
		return node;
	}
	let specializations = Specializations { mixed, operands };
	let variants = specializations.variants(&calls);
	specializations.with_copies(specializations.rewrite(node), &variants)
}

/// The functions defined at the top of the program whose parameters are all plain names
pub(super) fn copyable_functions(node: &Node) -> std::collections::HashSet<String> {
	let statements = match node.drop_meta() {
		Node::List(items, Bracket::None, _) => items.iter().collect(),
		single => vec![single],
	};
	statements.into_iter().filter_map(|statement| match statement.drop_meta() {
		Node::Key(head, Op::Define, _) => match head.drop_meta() {
			Node::List(items, Bracket::Round, _) if items.len() > 1 && items[1..].iter().all(|item| matches!(item.drop_meta(), Node::Symbol(_))) => Some(items[0].drop_meta().name()),
			_ => None,
		},
		_ => None,
	}).collect()
}

pub(super) struct Specializations<'a> {
	/// Per function, the positions whose calls give different classes
	mixed: std::collections::HashMap<String, Vec<bool>>,
	operands: &'a Operands<'a>,
}

impl Specializations<'_> {
	/// The classes a call gives at its function's mixed positions, when it gives one there
	fn variant(&self, function: &str, given: &[Option<String>]) -> Option<Vec<Option<String>>> {
		let mixed = self.mixed.get(function)?;
		let variant: Vec<Option<String>> = given.iter().zip(mixed).map(|(class, mixed)| class.clone().filter(|_| *mixed)).collect();
		variant.iter().any(Option::is_some).then_some(variant)
	}

	fn variants(&self, calls: &CallClasses) -> std::collections::HashMap<String, std::collections::BTreeSet<Vec<Option<String>>>> {
		let mut variants: std::collections::HashMap<String, std::collections::BTreeSet<Vec<Option<String>>>> = std::collections::HashMap::new();
		for (function, calls) in calls {
			variants.entry(function.clone()).or_default().extend(calls.iter().filter_map(|given| self.variant(function, given)));
		}
		variants
	}

	fn rewrite(&self, node: Node) -> Node {
		match node {
			Node::List(items, Bracket::Round, separator) if items.len() > 1 && self.mixed.contains_key(&items[0].drop_meta().name()) => {
				let function = items[0].drop_meta().name();
				let given: Vec<Option<String>> = items[1..].iter().map(|argument| argument_class(argument, self.operands)).collect();
				let head = match self.variant(&function, &given) {
					Some(variant) => Node::Symbol(variant_name(&function, &variant)),
					None => items[0].clone(),
				};
				let arguments = items[1..].iter().map(|argument| self.rewrite(argument.clone()));
				Node::List(std::iter::once(head).chain(arguments).collect(), Bracket::Round, separator)
			}
			other => other.map_children(|child| self.rewrite(child)),
		}
	}

	/// Each definition of a function called with instances, followed by its typed copies
	fn with_copies(&self, node: Node, variants: &std::collections::HashMap<String, std::collections::BTreeSet<Vec<Option<String>>>>) -> Node {
		match node {
			Node::Meta { node, data } => Node::Meta { node: Box::new(self.with_copies(*node, variants)), data },
			Node::List(items, Bracket::None, separator) => {
				let copies = |item: &Node| {
					let Node::Key(head, Op::Define, body) = item.drop_meta() else { return vec![] };
					let Node::List(parameters, Bracket::Round, head_separator) = head.drop_meta() else { return vec![] };
					let function = parameters[0].drop_meta().name();
					variants.get(&function).into_iter().flatten().map(|variant| {
						let typed = parameters[1..].iter().zip(variant).map(|(parameter, class)| match class {
							Some(class) => key(parameter.clone(), Op::Colon, Node::Symbol(class.clone())),
							None => parameter.clone(),
						});
						let head = Node::List(std::iter::once(Node::Symbol(variant_name(&function, variant))).chain(typed).collect(), Bracket::Round, head_separator.clone());
						Node::Key(Box::new(head), Op::Define, body.clone())
					}).collect()
				};
				let items = items.into_iter().flat_map(|item| { let copies = copies(&item); std::iter::once(item).chain(copies) }).collect();
				Node::List(items, Bracket::None, separator)
			}
			other => other,
		}
	}
}

/// `f_Quantity`, the copy of f for its classes at the mixed positions
pub(super) fn variant_name(function: &str, variant: &[Option<String>]) -> String {
	std::iter::once(function).chain(variant.iter().flatten().map(String::as_str)).collect::<Vec<_>>().join("_")
}

pub(super) fn definition_heads(node: &Node) -> Vec<Vec<Node>> {
	let mut heads = vec![];
	node.visit(&mut |part| if let Node::Key(head, Op::Define, _) = part {
		if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
			if items.len() > 1 {
				heads.push(items.clone());
			}
		}
	});
	heads
}

pub(super) fn collect_argument_classes(node: &Node, operands: &Operands, defined: &std::collections::HashSet<String>, calls: &mut CallClasses) {
	match node.drop_meta() {
		Node::Key(_, Op::Define, body) => collect_argument_classes(body, operands, defined, calls),
		Node::List(items, Bracket::Round, _) if items.len() > 1 && defined.contains(&items[0].drop_meta().name()) => {
			let given: Vec<Option<String>> = items[1..].iter().map(|argument| argument_class(argument, operands)).collect();
			calls.entry(items[0].drop_meta().name()).or_default().push(given);
			items[1..].iter().for_each(|argument| collect_argument_classes(argument, operands, defined, calls));
		}
		other => children_of(other).into_iter().for_each(|child| collect_argument_classes(child, operands, defined, calls)),
	}
}

/// The class of an argument that is an instance of a class with operator methods
pub(super) fn argument_class(argument: &Node, operands: &Operands) -> Option<String> {
	operand_class(argument, operands).filter(|class| operands.methods.iter().any(|(owner, _, _)| owner == class))
}

pub(super) fn children_of(node: &Node) -> Vec<&Node> {
	match node {
		Node::Key(left, _, right) => vec![left, right],
		Node::List(items, _, _) => items.iter().collect(),
		_ => vec![],
	}
}

/// What tells an operator's operand an instance: the classes' operator methods, the variables holding instances, the
/// functions returning them
pub(super) struct Operands<'a> {
	methods: &'a [(String, Op, String)],
	instances: &'a std::collections::HashMap<String, String>,
	returned: &'a std::collections::HashMap<String, String>,
	/// parameter_classes
	parameters: &'a std::collections::HashMap<String, Vec<Option<String>>>,
}

/// The node with its operator calls, and the class of the instance it gives when it is one
pub(super) fn with_operator_calls(node: Node, operands: &Operands) -> (Node, Option<String>) {
	let recurse = |child: Node| with_operator_calls(child, operands).0;
	match node {
		certainty if crate::uncertain::certainty_parts(&certainty).is_some() => (with_certain_amounts(&certainty, operands), None),
		sum if summed_quantities(&sum, operands).is_some() => {
			let quantities = with_operator_calls(summed_quantities(&sum, operands).expect("guarded").clone(), operands).0;
			(quantities_reduced(quantities), Some(crate::units::RUN_TIME_QUANTITY.to_string()))
		}
		Node::Key(left, op, right) if OPERATOR_METHODS.iter().any(|(known, _)| *known == op) => {
			let (left, class) = with_operator_calls(*left, operands);
			let (right, right_class) = with_operator_calls(*right, operands);
			let class = class.or_else(|| operand_class(&left, operands));
			let right_class = right_class.or_else(|| operand_class(&right, operands));
			// `"v: " + q` of a run-time quantity q joins its text, as a static quantity does (card quantity-falls)
			if op == Op::Add && (is_text(&left) || is_text(&right)) {
				return (key(joined_text(left, class), op, joined_text(right, right_class)), None);
			}
			let (left, class, right) = with_run_time_units(left, class, right, right_class, operands);
			match class.as_ref().and_then(|class| operands.methods.iter().find(|(owner, known, _)| owner == class && *known == op)) {
				Some((_, _, method)) => (method_call(left, method, right), class),
				None => (key(left, op, right), None),
			}
		}
		// `q as km/h`, `q in m` of a run-time quantity q: its conversion `q.to("km/h")` (card quantity-falls)
		conversion if crate::units::conversion(&conversion).is_some() => {
			let (quantity, units) = crate::units::conversion(&conversion).map(|(quantity, units)| (quantity.clone(), units)).expect("guarded");
			let (quantity, class) = with_operator_calls(quantity, operands);
			let run_time = Some(crate::units::RUN_TIME_QUANTITY);
			match class.or_else(|| operand_class(&quantity, operands)).as_deref() == run_time {
				true => (method_call(quantity, CONVERSION_METHOD, Node::Text(crate::units::unit_suffix(&units))), run_time.map(str::to_string)),
				false => (conversion.map_children(recurse), None),
			}
		}
		// `1 min == q` of a run-time quantity q (the equality witness compares them)
		Node::Key(left, op @ (Op::Eq | Op::Ne), right) => {
			let (left, class) = with_operator_calls(*left, operands);
			let (right, right_class) = with_operator_calls(*right, operands);
			let class = class.or_else(|| operand_class(&left, operands));
			let (left, _, right) = with_run_time_units(left, class, right, right_class, operands);
			(key(left, op, right), None)
		}
		Node::List(items, Bracket::Round, separator) if items.len() == 1 => {
			let (item, class) = with_operator_calls(items[0].clone(), operands);
			(Node::List(vec![item], Bracket::Round, separator), class)
		}
		// `f(a) := a + 1`: its parameters are none of the program's instance variables of the same names; one typed by a
		// class (`q:Quantity`), or given instances of one class by every call, is an instance of it
		Node::Key(head, Op::Define, body) if matches!(head.drop_meta(), Node::List(items, Bracket::Round, _) if items.len() > 1) => {
			let Node::List(items, _, _) = head.drop_meta() else { unreachable!("guarded") };
			let mut instances = operands.instances.clone();
			let called_with = operands.parameters.get(&items[0].drop_meta().name());
			for (position, parameter) in items[1..].iter().enumerate() {
				let (name, declared) = match parameter.drop_meta() {
					Node::Key(name, Op::Colon, type_node) => (name.drop_meta().name(), Some(type_node.drop_meta().name())),
					other => (other.name(), called_with.and_then(|classes| classes.get(position).cloned().flatten())),
				};
				match declared.filter(|class| operands.methods.iter().any(|(owner, _, _)| owner == class)) {
					Some(class) => instances.insert(name, class),
					None => instances.remove(&name),
				};
			}
			let body = with_operator_calls(*body, &Operands { instances: &instances, ..*operands }).0;
			(Node::Key(head, Op::Define, Box::new(body)), None)
		}
		other => (other.map_children(recurse), None),
	}
}

/// `receiver.method(argument)`; `(quantity(…)) * 2`: the receiver without its parentheses, else `(f(…)).times` reads as
/// a call of f
/// `rope certainly > 4 m` of a run-time quantity rope: its amount and the other's in the same base units compare, a ±
/// amount as the interval it is; `Quantity.more` would answer a plain yes (card quantity-tolerance)
pub(super) fn with_certain_amounts(certainty: &Node, operands: &Operands) -> Node {
	let (word, ordering) = crate::uncertain::certainty_parts(certainty).expect("guarded");
	let Node::Key(left, op, right) = ordering.drop_meta() else { unreachable!("certainty_parts takes orderings") };
	let (left, class) = with_operator_calls(left.as_ref().clone(), operands);
	let (right, right_class) = with_operator_calls(right.as_ref().clone(), operands);
	let class = class.or_else(|| operand_class(&left, operands));
	let (left, class, right) = with_run_time_units(left, class, right, right_class, operands);
	let compared = match class.as_deref() == Some(crate::units::RUN_TIME_QUANTITY) {
		true => {
			let comparable = call(SAME_DIMENSION, vec![left.clone(), right, text("compare")]);
			key(amount_of(left), *op, amount_of(comparable))
		}
		false => key(left, *op, right),
	};
	Node::List(vec![symbol(word), compared], Bracket::None, Separator::Space).with_meta_of(certainty)
}

/// The list of `sum([5 m ± 1 cm, 3 m ± 2 cm])` or `[…].sum()` when each item is a run-time quantity
pub(super) fn summed_quantities<'n>(node: &'n Node, operands: &Operands) -> Option<&'n Node> {
	let list = summed_list(node)?;
	let Node::List(items, Bracket::Square, _) = list.drop_meta() else { return None };
	let is_quantity = |item: &Node| operand_class(item, operands).as_deref() == Some(crate::units::RUN_TIME_QUANTITY);
	(!items.is_empty() && items.iter().all(is_quantity)).then_some(list)
}

/// The list of `sum(xs)`, `xs.sum()`
pub(super) fn summed_list(node: &Node) -> Option<&Node> {
	match node.drop_meta() {
		Node::List(items, Bracket::Round, _) => match items.as_slice() {
			[word, list] if word.drop_meta().name() == SUM_WORD => Some(list),
			_ => None,
		},
		Node::Key(list, Op::Dot, call) if is_call_of(call, SUM_WORD) || call.drop_meta().name() == SUM_WORD => Some(list),
		_ => None,
	}
}

/// `quantities.reduce((sum, item) => sum.plus(item))`, marked a Quantity so `str()` and a final value show its text
/// (card quantity-sum)
pub(super) fn quantities_reduced(quantities: Node) -> Node {
	let [sum, item] = REDUCED_NAMES.map(symbol);
	let parameters = Node::List(vec![sum.clone(), item.clone()], Bracket::Round, Separator::Colon);
	let added = key(parameters, Op::FatArrow, method_call(sum, operator_method(Op::Add).expect("plus"), item));
	let reduced = key(quantities, Op::Dot, call(REDUCE_WORD, vec![added]));
	Node::meta(reduced, Node::data(crate::lowering::traits::TypedAs(crate::units::RUN_TIME_QUANTITY.to_string())))
}

pub(super) fn amount_of(quantity: Node) -> Node {
	key(quantity, Op::Dot, symbol(QUANTITY_AMOUNT))
}

pub(super) fn method_call(receiver: Node, method: &str, argument: Node) -> Node {
	let receiver = match receiver.drop_meta() {
		Node::List(items, Bracket::Round, _) if items.len() == 1 => items[0].clone(),
		_ => receiver,
	};
	let call = call(method, vec![argument]);
	key(receiver, Op::Dot, call)
}

/// `2 km < q`, `q + 1 m` of a run-time quantity q: the unit written in the program is one too (card units-mixed); the
/// operands with the left one's class
pub(super) fn with_run_time_units(left: Node, class: Option<String>, right: Node, right_class: Option<String>, operands: &Operands) -> (Node, Option<String>, Node) {
	let run_time = Some(crate::units::RUN_TIME_QUANTITY);
	let lifted_left = class.is_none() && right_class.or_else(|| operand_class(&right, operands)).as_deref() == run_time;
	let (left, class) = match lifted_left.then(|| crate::units::as_run_time_quantity(&left)).flatten() {
		Some(quantity) => (quantity, run_time.map(str::to_string)),
		None => (left, class),
	};
	let right = match class.as_deref() == run_time {
		true => crate::units::as_run_time_quantity(&right).unwrap_or(right),
		false => right,
	};
	(left, class, right)
}

/// The class of the instance an operand is: a variable holding one, a parenthesized operation, a construction or a call
/// of a function returning one
pub(super) fn operand_class(operand: &Node, operands: &Operands) -> Option<String> {
	match operand.drop_meta() {
		Node::Symbol(variable) => operands.instances.get(variable).cloned(),
		// `(a + b).x`: the parenthesized operation, `(quantity("5 m")) * 2` the parenthesized instance
		Node::List(items, Bracket::Round, _) if items.len() == 1 => with_operator_calls(items[0].clone(), operands).1.or_else(|| operand_class(&items[0], operands)),
		// `f(quantity("5 m") / 5)`: an operation of an instance as an argument
		Node::Key(_, op, _) if OPERATOR_METHODS.iter().any(|(known, _)| known == op) => with_operator_calls(operand.drop_meta().clone(), operands).1,
		// `q.times(2)`: an operator method already called on an instance gives one of its class
		Node::Key(receiver, Op::Dot, call) if is_operator_method_call(call, operands) || is_call_of(call, CONVERSION_METHOD) => operand_class(receiver, operands),
		other => instance_class(other, operands.returned),
	}
}

pub(super) fn is_text(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Text(_))
}

/// An operand joining a text: a run-time quantity as `str(q)`
pub(super) fn joined_text(operand: Node, class: Option<String>) -> Node {
	match class.as_deref() == Some(crate::units::RUN_TIME_QUANTITY) {
		true => call(TEXT_WORD, vec![operand]),
		false => operand,
	}
}

pub(super) fn is_operator_method_call(call: &Node, operands: &Operands) -> bool {
	operands.methods.iter().any(|(_, _, method)| is_call_of(call, method))
}

/// `to("m")` is a call of `to`
pub(super) fn is_call_of(call: &Node, method: &str) -> bool {
	matches!(call.drop_meta(), Node::List(items, Bracket::Round, _) if items.first().is_some_and(|word| word.drop_meta().name() == method))
}

/// A name the library gives lists and texts too (`pop`, `sum`, `count`): a method of that name is one only on an
/// instance
pub(super) fn is_library_method(name: &str) -> bool {
	crate::analyzer::is_list_mutating_method(name) || crate::library_words::is_library_word(name) || crate::analyzer::is_counting_property(name)
}
