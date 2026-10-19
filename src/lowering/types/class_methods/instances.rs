//! Instances: `object as Point` and its checked cast, and which variables and functions hold an instance of which class
use super::*;
use crate::lowering::nodes::assign;
use crate::node::parens;

pub(super) fn with_objects_as_instances(node: Node, fields: &ClassFields) -> Node {
	let class_of = |node: &Node| Some(node.drop_meta().name()).filter(|name| fields.contains_key(name));
	match node {
		// `parse_json(t) as [Point]`: each element an instance
		Node::Key(object, Op::As, list) if matches!(list.drop_meta(), Node::List(items, Bracket::Square, _) if items.len() == 1 && class_of(&items[0]).is_some()) =>
		{
			let class = list.drop_meta().children()[0].drop_meta().name();
			instances_of(with_objects_as_instances(*object, fields), &class, fields)
		}
		// only an object that is no instance yet: `render(t) as docx` picks render's overload (overloads.rs)
		Node::Key(object, Op::As, class) if class_of(&class).is_some() && is_plain_object(&object) => {
			let class = class.drop_meta().name();
			instance_from(with_objects_as_instances(*object, fields), &class, fields)
		}
		// `s as Circle` of a variable or field: the checked downcast (P201); `render(t) as docx` of a call picks an
		// overload (overloads.rs)
		Node::Key(object, Op::As, class) if class_of(&class).is_some() && is_place(&object) => {
			checked_cast(with_objects_as_instances(*object, fields), &class.drop_meta().name())
		}
		Node::Key(class, Op::Dot, member) if class_of(&class).is_some() && leading_name(&member) == FROM_JSON_WORD => {
			let arguments = match member.drop_meta() {
				Node::List(items, Bracket::Round, _) => {
					items[1..].iter().cloned().map(|argument| with_objects_as_instances(argument, fields)).collect()
				}
				_ => vec![],
			};
			let parsed = call(PARSE_JSON_WORD, arguments);
			let class = class.drop_meta().name();
			instance_from(parsed, &class, fields)
		}
		other => other.map_children(|child| with_objects_as_instances(child, fields)),
	}
}

/// `(cast·value = s; if not (cast·value is Circle) { raise "s is no Circle" }; cast·value)`: the value is computed once
pub(super) fn checked_cast(object: Node, class: &str) -> Node {
	let held = symbol(CAST_VALUE);
	let message = format!("{} is no {class}", object.serialize());
	let check =
		crate::warp_parser::parse_generated(&format!("if not ({CAST_PLACEHOLDER} is {class}) {{ raise {message:?} }}"));
	let statements =
		vec![assign(held.clone(), object), crate::library_words::substitute(check, CAST_PLACEHOLDER, &held), held];
	Node::List(statements, Bracket::Round, Separator::Semicolon)
}

pub(super) fn instance_from(object: Node, class: &str, fields: &ClassFields) -> Node {
	let source = Node::Symbol(format!("{class}{FROM_SUFFIX}"));
	let mut statements = vec![assign(source.clone(), object)];
	statements.extend(built_instance(&source, class, fields).body_statements());
	Node::List(statements, Bracket::Round, Separator::Semicolon)
}

/// The construction of `instance_of`, after the lists of instances it takes (a construction's arguments are data:
/// the `map` building a list runs before, into `elements·1`, …)
pub(super) fn built_instance(path: &Node, class: &str, fields: &ClassFields) -> Node {
	let mut lists = vec![];
	let construction = instance_of(path, class, fields, &mut lists);
	match lists.is_empty() {
		true => construction,
		false => Node::List([lists, vec![construction]].concat(), Bracket::Round, Separator::Semicolon),
	}
}

/// `Point(path.x, path.y)`, a field of a class type by its own path, a list of instances by its variable in `lists`
pub(super) fn instance_of(path: &Node, class: &str, fields: &ClassFields, lists: &mut Vec<Node>) -> Node {
	let mut values = vec![];
	for (field, field_type) in &fields[class] {
		let value = key(path.clone(), Op::Dot, Node::Symbol(field.clone()));
		let element_class = field_type
			.strip_prefix('[')
			.and_then(|inner| inner.strip_suffix(']'))
			.filter(|inner| fields.contains_key(*inner));
		values.push(match (fields.contains_key(field_type), element_class) {
			(true, _) => instance_of(&value, field_type, fields, lists),
			(false, Some(element_class)) => {
				let elements = Node::Symbol(format!("{ELEMENTS_WORD}·{}", lists.len() + 1));
				lists.push(assign(elements.clone(), instances_of(value, element_class, fields)));
				elements
			}
			(false, None) => value,
		});
	}
	parens(std::iter::once(symbol(class)).chain(values).collect())
}

/// `list.map(Point·element => Point(Point·element.x, …))`
pub(super) fn instances_of(list: Node, class: &str, fields: &ClassFields) -> Node {
	let element = Node::Symbol(format!("{class}{ELEMENT_SUFFIX}"));
	let lambda = key(element.clone(), Op::Arrow, built_instance(&element, class, fields));
	key(list, Op::Dot, call(MAP_WORD, vec![lambda]))
}

/// The variables known to hold an instance of one of `classes`, with its class: assigned a construction or a call of a
/// function returning one, annotated `p:Point`, or iterating a list of constructions `for p in ps`
pub(crate) fn instance_classes(node: &Node, classes: &[String]) -> std::collections::HashMap<String, String> {
	let finder = InstanceFinder { classes, returned: returned_classes(node, classes) };
	let mut instances: std::collections::HashMap<String, String> = std::collections::HashMap::new();
	let mut assigned = std::collections::HashSet::new();
	let mut ambiguous = std::collections::HashSet::new();
	node.visit(&mut |part| {
		let Node::Key(target, op, value) = part else { return };
		if let (Some(variable), Op::Assign) = (target.symbol_name(), op) {
			assigned.insert(variable.to_string());
		}
		// `p:Point = …`: the annotation says it
		if let (Node::Key(variable, Op::Colon, class), Op::Assign) = (target.drop_meta(), op) {
			// `p:Point`, not a list of them `ps:[Point]`
			let is_class = matches!(class.drop_meta(), Node::Symbol(_) | Node::Type { .. });
			if is_class && classes.contains(&class.drop_meta().name()) {
				instances.insert(variable.drop_meta().name(), class.drop_meta().name());
			}
		}
		let Node::Symbol(variable) = target.drop_meta() else {
			return;
		};
		if let Some(class) = finder.assigned_class(op, value, &instances).filter(|class| classes.contains(class)) {
			if instances.insert(variable.clone(), class.clone()).is_some_and(|before| before != class) {
				ambiguous.insert(variable.clone());
			}
		}
	});
	// `b = as_interval(o)` in one method, `b = as_gaussian(o)` in another: b is neither class
	instances.retain(|variable, _| !ambiguous.contains(variable));
	finder.iterated(node, &mut instances);
	let module_instances = crate::compiler::compile_context::read(|context| context.module_instance_classes.clone());
	for (variable, class) in module_instances {
		// a program's own `class B` or `B = …` hides sound's note B
		if classes.contains(&class) && !assigned.contains(&variable) && !classes.contains(&variable) {
			instances.entry(variable).or_insert(class);
		}
	}
	instances
}

/// The variables a program's top level assigns an instance of one of `classes`, with its class: `F = PitchClass(5)`
pub(crate) fn top_level_instances(node: &Node, classes: &[String]) -> std::collections::HashMap<String, String> {
	node.as_items()
		.iter()
		.filter_map(|statement| match statement.drop_meta() {
			Node::Key(variable, Op::Assign, value) => {
				let class = constructed_class(value).filter(|class| classes.contains(class))?;
				Some((variable.symbol_name()?.to_string(), class))
			}
			_ => None,
		})
		.collect()
}

struct InstanceFinder<'a> {
	classes: &'a [String],
	/// returned_classes
	returned: std::collections::HashMap<String, String>,
}

impl InstanceFinder<'_> {
	fn constructed_class(&self, value: &Node) -> Option<String> {
		instance_class(value, &self.returned)
	}

	/// `[Point(1, 2), …]`: a list of constructions of one class
	fn list_class(&self, list: &Node) -> Option<String> {
		match list.drop_meta() {
			Node::List(items, Bracket::Square, _) if !items.is_empty() => {
				let first = self.constructed_class(&items[0])?;
				items.iter().all(|item| self.constructed_class(item).as_ref() == Some(&first)).then_some(first)
			}
			_ => None,
		}
	}

	/// The class of the instance `op value` gives a variable: `=` a construction, an operation or conversion of an
	/// instance, a sum of instances; `:` an annotation
	fn assigned_class(
		&self,
		op: &Op,
		value: &Node,
		instances: &std::collections::HashMap<String, String>,
	) -> Option<String> {
		let class_of = |operand: &Node| match operand.drop_meta() {
			Node::Symbol(operand) => instances.get(operand).cloned(),
			other => self.constructed_class(other),
		};
		match (op, value.drop_meta()) {
			// `c = a + b` of an instance a: the operation gives one of a's class, as with_operator_calls reads it
			(Op::Assign | Op::Define, Node::Key(left, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod, _)) => {
				class_of(left)
			}
			// `w = q as m`, `w = q.to("m")` of a run-time quantity q: another one
			(Op::Assign | Op::Define, conversion) if crate::numbers::units::conversion(conversion).is_some() => {
				crate::numbers::units::conversion(conversion).and_then(|(quantity, _)| class_of(quantity))
			}
			(Op::Assign | Op::Define, Node::Key(quantity, Op::Dot, call)) if call.is_call_of(CONVERSION_METHOD) => {
				class_of(quantity)
			}
			// `s = sum([quantity("5 m"), …])`: the sum of instances is one (card quantity-sum)
			(Op::Assign | Op::Define, summed) if summed_list(summed).is_some() => {
				summed_list(summed).and_then(|list| self.list_class(list))
			}
			(Op::Assign | Op::Define, _) => self.constructed_class(value),
			// `p:Point`, the annotation a symbol or a type
			(Op::Colon, Node::Symbol(_) | Node::Type { .. }) => Some(value.drop_meta().name()),
			_ => None,
		}
	}

	/// `for p in ps {…}` over a list of constructions `ps = [Point(1, 2), …]`: p holds instances
	fn iterated(&self, node: &Node, instances: &mut std::collections::HashMap<String, String>) {
		let mut instance_lists = std::collections::HashMap::new();
		node.visit(&mut |part| {
			if let Node::Key(target, Op::Assign | Op::Define, value) = part {
				if let Some(class) = self.list_class(value).filter(|class| self.classes.contains(class)) {
					instance_lists.insert(target.drop_meta().name(), class);
				}
			}
		});
		node.visit(&mut |part| {
			if let Node::List(words, _, _) = part {
				if let [for_word, item, in_word, list, ..] = words.as_slice() {
					let class = instance_lists
						.get(&list.drop_meta().name())
						.cloned()
						.or_else(|| self.list_class(list))
						.filter(|class| self.classes.contains(class));
					if let (true, Some(class)) =
						(for_word.drop_meta().name() == "for" && in_word.drop_meta().name() == "in", class)
					{
						instances.insert(item.drop_meta().name(), class);
					}
				}
			}
		});
	}
}

/// The functions whose value is an instance of one of `classes`, with its class: `make(n) := V(n)`, a body ending in
/// a construction, or in a call of another such function, or declaring the class as its result `make(n): V := …`; a
/// used module's such functions too (module_returned_classes)
pub(crate) fn returned_classes(node: &Node, classes: &[String]) -> std::collections::HashMap<String, String> {
	let mut definitions = vec![];
	node.visit(&mut |part| {
		if let Node::Key(head, Op::Define, body) = part {
			if let Node::List(items, Bracket::Round, _) = head.drop_meta() {
				definitions.extend(items.first().map(|name| (name.drop_meta().name(), last_value(body))));
			}
		}
	});
	// the program's functions only: a class's methods (`plus(o): Interval` of Interval and Gaussian) are no such calls
	let declared = crate::modules::statements_of(node).iter().filter_map(|statement| {
		let Node::Key(target, Op::Define, _) = statement.drop_meta() else { return None };
		let (head, result) = crate::lowering::nodes::result_typed(target)?;
		let Node::List(items, Bracket::Round, _) = head.drop_meta() else { return None };
		let class = Some(result.drop_meta().name()).filter(|class| classes.contains(class))?;
		Some((items.first()?.drop_meta().name(), class))
	});
	let mut returned: std::collections::HashMap<String, String> =
		crate::compiler::compile_context::read(|context| context.module_returned_classes.clone())
			.into_iter()
			.filter(|(function, class)| classes.contains(class) && !definitions.iter().any(|(own, _)| own == function))
			.chain(declared)
			.collect();
	// a function returning another's result: until no function is added
	loop {
		let known = returned.len();
		for (function, value) in &definitions {
			let class = constructed_class(value).and_then(|name| {
				if classes.contains(&name) {
					Some(name)
				} else {
					returned.get(&name).cloned()
				}
			});
			if let Some(class) = class {
				returned.entry(function.clone()).or_insert(class);
			}
		}
		if returned.len() == known {
			return returned;
		}
	}
}

/// The class of an instance a construction or a call of a function of `returned` gives
pub(super) fn instance_class(value: &Node, returned: &std::collections::HashMap<String, String>) -> Option<String> {
	constructed_class(value).map(|name| returned.get(&name).cloned().unwrap_or(name))
}

/// The value of a body: its last statement
pub(super) fn last_value(body: &Node) -> &Node {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, _) if !items.is_empty() => last_value(&items[items.len() - 1]),
		other => other,
	}
}

/// The class a construction `Point(1, 2)`, `Point{…}` builds (by its name, before the constructions are lowered)
pub(super) fn constructed_class(value: &Node) -> Option<String> {
	match value.drop_meta() {
		Node::List(items, Bracket::Round, _) => items.first().map(|first| first.drop_meta().name()),
		Node::Key(class, Op::None | Op::Colon, _) => Some(class.drop_meta().name()),
		_ => None,
	}
}
