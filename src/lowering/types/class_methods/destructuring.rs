//! Destructuring and patterns of instances: `{x, y} = p`, `(a, b) = p`, `c#1` and `c.0`, `Point{x, y: b}` patterns
use super::*;
use crate::lowering::node_cases;
use crate::lowering::nodes::assign;

/// Destructuring (JS, Python, Rust, Kotlin): `{x, y} = p` and `{x: a} = p` by field name (maps too); `(a, b) = p` and
/// `a, b = p` of an instance in field order; match arms `Point{x, y} => …` and `Point(x, y) => …` a type test binding
/// the fields
pub(super) fn destructurings(node: Node) -> Node {
	let cases = enum_cases(&node);
	with_value_classes(node, &|node, fields, class_of| taken_apart(node, fields, class_of, &cases))
}

/// The cases of the program's enums without values, `Red` of `enum Color { Red, Green }`: a pattern compares them
fn enum_cases(node: &Node) -> Cases {
	crate::lowering::sum_variants::sum_types(node)
		.values()
		.flat_map(|variants| variants.bare_names().collect::<Vec<_>>())
		.collect()
}

pub(super) type Cases = std::collections::HashSet<String>;

/// P179: a field by its position, `c#1` 1-based and Rust's `c.0` 0-based, of an instance whose class is known
/// (`c = rgb(1, 2, 3)`, `Some(4)#1`): the field by name, `c.value1`; `c#1` of a class defining `#` calls that method
pub(super) fn positional_fields(node: Node) -> Node {
	let indexing = indexing_classes(&node);
	with_value_classes(node, &|node, fields, class_of| by_position(node, fields, class_of, &indexing))
}

/// The classes that define the index operator `#` as a method (`index(n) := …`)
fn indexing_classes(node: &Node) -> std::collections::HashSet<String> {
	let mut classes = std::collections::HashSet::new();
	node.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			if class_items(body)
				.iter()
				.filter_map(method_parts)
				.any(|(method, _, _)| Some(method.as_str()) == operator_method(Op::Hash))
			{
				classes.insert(name.drop_meta().name());
			}
		}
	});
	classes
}

/// `lower` of the program with each class's fields and the class of a value: a variable's instance class or the class
/// a construction makes
pub(super) fn with_value_classes(node: Node, lower: &dyn Fn(Node, &ClassFields, &ClassOf) -> Node) -> Node {
	let fields = class_fields(&node);
	let classes: Vec<String> = fields.keys().cloned().collect();
	let instances = instance_classes(&node, &classes);
	let class_of = |value: &Node| match value.drop_meta() {
		Node::Symbol(variable) => instances.get(variable).cloned(),
		other => constructed_class(other).filter(|class| fields.contains_key(class)),
	};
	lower(node, &fields, &class_of)
}

fn by_position(
	node: Node,
	fields: &ClassFields,
	class_of: &ClassOf,
	indexing: &std::collections::HashSet<String>,
) -> Node {
	let recurse = |child: Node| by_position(child, fields, class_of, indexing);
	let position = |op: Op, index: &Node| match (op, index.drop_meta()) {
		(Op::Hash, Node::Number(crate::Number::Int(number))) => Some(number - 1),
		(Op::Dot, Node::Number(crate::Number::Int(number))) => Some(*number),
		_ => None,
	};
	// `a: int = Some(3)`: a variable of a built-in type takes the single field of the construction it is given
	let unwrapped_field = |declared: &Node, value: &Node| match (
		declared.drop_meta(),
		constructed_class(value).and_then(|class| fields.get(&class)),
	) {
		(Node::Key(_, Op::Colon, kind), Some(class_fields))
			if class_fields.len() == 1 && crate::analyzer::builtin_type_kind(&kind.drop_meta().name()).is_some() =>
		{
			Some(class_fields[0].0.clone())
		}
		_ => None,
	};
	match node {
		Node::Key(declared, Op::Assign, value) if unwrapped_field(&declared, &value).is_some() => {
			let field = unwrapped_field(&declared, &value).expect("guarded");
			Node::Key(declared, Op::Assign, Box::new(field_of(&recurse(*value), &field)))
		}
		Node::Key(instance, op @ (Op::Hash | Op::Dot), index)
			if position(op, &index).is_some()
				&& class_of(&instance).is_some_and(|class| op == Op::Dot || !indexing.contains(&class)) =>
		{
			let class = class_of(&instance).expect("guarded");
			let class_fields = &fields[&class];
			let instance = recurse(*instance);
			match usize::try_from(position(op, &index).expect("guarded")).ok().and_then(|at| class_fields.get(at)) {
				Some((field, _)) => field_of(&instance, field),
				None => crate::node::error(&format!(
					"{} has {} fields: {} is out of range",
					class,
					class_fields.len(),
					index.serialize().trim()
				)),
			}
		}
		other => other.map_children(recurse),
	}
}

pub(super) fn taken_apart(node: Node, fields: &ClassFields, class_of: &ClassOf, cases: &Cases) -> Node {
	let recurse = |child: Node| taken_apart(child, fields, class_of, cases);
	let subject = symbol(PARTS_WORD);
	match node {
		// `{x, start: Point{x: a}} = l`: names only, no constant to test (a class in it is no test either)
		Node::Key(target, Op::Assign, value)
			if matched_fields(&target, &subject, fields, cases).is_some_and(|matched| matched.tests.is_empty()) =>
		{
			let matched = matched_fields(&target, &subject, fields, cases).expect("guarded");
			bound_parts(recurse(*value), &matched.bindings, None)
		}
		Node::Key(target, Op::Assign, value) if positional_names(&target).is_some() && class_of(&value).is_some() => {
			let names = positional_names(&target).expect("guarded");
			let class = class_of(&value).expect("guarded");
			if fields[&class].len() != names.len() {
				return Node::Key(target, Op::Assign, value);
			}
			let bindings = fields[&class]
				.iter()
				.zip(names)
				.map(|((field, _), name)| (field_of(&subject, field), name))
				.collect::<Vec<_>>();
			bound_parts(recurse(*value), &bindings, None)
		}
		// Python's `a, b = p`: the parser reads `a, (b = p)`
		Node::List(items, bracket, separator)
			if separator == Separator::Comma && last_assigns_instance(&items, class_of) =>
		{
			let Some(Node::Key(last, Op::Assign, value)) = items.last().map(|item| item.drop_meta().clone()) else {
				return Node::List(items, bracket, separator);
			};
			let names = [&items[..items.len() - 1], &[*last]].concat();
			let target = Node::List(names, Bracket::Round, Separator::Comma);
			recurse(Node::Key(Box::new(target), Op::Assign, value))
		}
		Node::List(items, bracket, separator) if is_match(&items) => {
			let items = crate::welcome_forms::with_match_cases(items, |arms| class_arms(arms, fields, cases));
			Node::List(items, bracket, separator).map_children(recurse)
		}
		other => other.map_children(recurse),
	}
}

fn is_match(items: &[Node]) -> bool {
	items.first().is_some_and(|word| crate::words::SWITCH_WORDS.iter().any(|switch| word.is_symbol(switch)))
}

/// The class arms of a match's cases, `Point{x, y: 0}: body` and `Circle(r): body`, as the guard
/// `if parts·from is Point and parts·from.y == 0 then parts·from` and the body with the fields bound
fn class_arms(arms: &Node, fields: &ClassFields, cases: &Cases) -> Node {
	let Node::List(arms, Bracket::Curly, separator) = arms.drop_meta() else {
		return arms.clone();
	};
	let arms = arms.iter().map(|arm| class_arm(arm, fields, cases).unwrap_or_else(|| arm.clone()));
	Node::List(arms.collect(), Bracket::Curly, separator.clone())
}

fn class_arm(arm: &Node, fields: &ClassFields, cases: &Cases) -> Option<Node> {
	let Node::Key(pattern, Op::Colon, body) = arm.drop_meta() else {
		return None;
	};
	let (subject, mut matched) = (symbol(PARTS_WORD), Matched::default());
	is_class_pattern(pattern, fields).then_some(())?;
	matched_pattern(pattern, &subject, fields, cases, &mut matched)?;
	// `is` compares like ==, a class name tests the type
	let test = matched.class_tests.into_iter().chain(matched.tests).reduce(|all, test| key(all, Op::And, test));
	let guard = if_then(test.expect("a class pattern tests its class"), subject.clone());
	let body = bound_parts(subject, &matched.bindings, Some(body.as_ref().clone()));
	Some(key(guard, Op::Colon, body))
}

/// What a pattern asks of its subject: class tests (`p is Point`), value tests (`p.x == 0`) and the variables it
/// binds, each with its path
#[derive(Default)]
pub(super) struct Matched {
	class_tests: Vec<Node>,
	tests: Vec<Node>,
	bindings: Vec<(Node, String)>,
}

pub(super) fn field_of(path: &Node, field: &str) -> Node {
	key(path.clone(), Op::Dot, symbol(field))
}

/// `Point{…}` or `Point(…)` of a declared class
pub(super) fn is_class_pattern(pattern: &Node, fields: &ClassFields) -> bool {
	match pattern.drop_meta() {
		Node::Key(class, Op::Colon | Op::None, parts) => {
			fields.contains_key(&class.drop_meta().name()) && parts.is_block()
		}
		Node::List(items, Bracket::Round, _) => {
			items.len() > 1
				&& (fields.contains_key(&items[0].drop_meta().name()) || built_in_case(items, fields).is_some())
		}
		_ => false,
	}
}

/// A pattern at `path`: `_` matches anything, a name binds, a number or text is compared, `Point{x, y: b}` and
/// `Point(a, 0)` test the class and match each field; None for anything else (the arm stays as written)
pub(super) fn matched_pattern(
	pattern: &Node,
	path: &Node,
	fields: &ClassFields,
	cases: &Cases,
	matched: &mut Matched,
) -> Option<()> {
	let class_test = |class: &str| key(path.clone(), Op::Eq, symbol(class));
	match pattern.drop_meta() {
		Node::Symbol(name) if name == "_" => {}
		// `Color.Red`, a case of an enum (Red by then), compares
		Node::Symbol(name) if cases.contains(name) => {
			matched.tests.push(key(path.clone(), Op::Eq, pattern.drop_meta().clone()))
		}
		Node::Symbol(name) if !fields.contains_key(name) => matched.bindings.push((path.clone(), name.clone())),
		Node::Number(_) | Node::Text(_) | Node::Char(_) | Node::True | Node::False => {
			matched.tests.push(key(path.clone(), Op::Eq, pattern.drop_meta().clone()))
		}
		Node::Key(class, Op::Colon | Op::None, parts) if fields.contains_key(&class.drop_meta().name()) => {
			matched.class_tests.push(class_test(&class.drop_meta().name()));
			let inner = matched_fields(parts, path, fields, cases)?;
			matched.class_tests.extend(inner.class_tests);
			matched.tests.extend(inner.tests);
			matched.bindings.extend(inner.bindings);
		}
		Node::List(items, Bracket::Round, _)
			if items.len() > 1 && fields.contains_key(&items[0].drop_meta().name()) =>
		{
			let class = items[0].drop_meta().name();
			let class_fields = &fields[&class];
			let parts = &items[1..];
			// `Point(x, ..)`: the parts before an unnamed rest take the first fields, those after it the last ones
			let (before, after) =
				match parts.iter().position(|part| super::super::switch::rest_pattern(part) == Some(None)) {
					Some(rest) if parts.len() - 1 <= class_fields.len() => (&parts[..rest], &parts[rest + 1..]),
					Some(_) => return None,
					None if parts.len() == class_fields.len() => (parts, &[][..]),
					None => return None,
				};
			matched.class_tests.push(class_test(&class));
			let last_fields = &class_fields[class_fields.len() - after.len()..];
			for ((field, _), part) in class_fields.iter().zip(before).chain(last_fields.iter().zip(after)) {
				matched_pattern(part, &field_of(path, field), fields, cases, matched)?;
			}
		}
		// `Key(left, op, right)` of the built-in node (node_cases.rs): its kind tested, its parts read
		Node::List(items, Bracket::Round, _) if built_in_case(items, fields).is_some() => {
			let case = built_in_case(items, fields).expect("guarded");
			matched.class_tests.push(node_cases::kind_test(case, path));
			for (part, pattern) in case.fields.iter().zip(&items[1..]) {
				matched_pattern(pattern, &node_cases::field_at(part, path), fields, cases, matched)?;
			}
		}
		_ => return None,
	}
	Some(())
}

/// The built-in node's case a pattern `Key(left, op, right)` names, one field for each of its parts, when the program
/// declares no class of that name
fn built_in_case(items: &[Node], fields: &ClassFields) -> Option<&'static node_cases::NodeCase> {
	let name = items.first()?.drop_meta().name();
	let case = node_cases::node_case(&name).filter(|_| !fields.contains_key(&name))?;
	(case.fields.len() == items.len() - 1).then_some(case)
}

/// `{x, y: b, from: Point{…}}` at `path`: each field by name, a field with a pattern after its colon matched by it
pub(super) fn matched_fields(parts: &Node, path: &Node, fields: &ClassFields, cases: &Cases) -> Option<Matched> {
	let Node::List(items, Bracket::Curly, _) = parts.drop_meta() else {
		return None;
	};
	if items.is_empty() {
		return None;
	}
	let mut matched = Matched::default();
	for item in items {
		match item.drop_meta() {
			Node::Symbol(name) => matched.bindings.push((field_of(path, name), name.clone())),
			Node::Key(field, Op::Colon, part) if matches!(field.drop_meta(), Node::Symbol(_)) => {
				matched_pattern(part, &field_of(path, &field.drop_meta().name()), fields, cases, &mut matched)?
			}
			// Rust's `Point{x, ..}`: the other fields left out
			_ if super::super::switch::rest_pattern(item) == Some(None) => {}
			_ => return None,
		}
	}
	Some(matched)
}

/// `(parts·from = value; x = parts·from.x; …; body)`
pub(super) fn bound_parts(value: Node, bindings: &[(Node, String)], body: Option<Node>) -> Node {
	let subject = symbol(PARTS_WORD);
	let mut statements = match value.drop_meta() {
		Node::Symbol(name) if name == PARTS_WORD => vec![],
		_ => vec![assign(subject, value)],
	};
	// an arm's names are its own locals, as a match arm's (card param-pattern-local-warning); `{x, y} = p` assigns
	let assigned = |(path, name): &(Node, String)| match body {
		Some(_) => crate::lowering::switch::bind(Node::Symbol(name.clone()), path.clone()),
		None => assign(Node::Symbol(name.clone()), path.clone()),
	};
	statements.extend(bindings.iter().map(assigned));
	statements.extend(body);
	Node::List(statements, Bracket::Round, Separator::Semicolon)
}

/// `(a, b)`: the names in order; a glued `long(n)` is the head of a function (`def long(n) = Run(…)`), no names
pub(super) fn positional_names(target: &Node) -> Option<Vec<String>> {
	let Node::List(items, Bracket::Round, separator) = target.drop_meta() else {
		return None;
	};
	(*separator != Separator::None).then_some(())?;
	(items.len() > 1).then_some(())?;
	items.iter().map(|item| item.symbol_name().map(String::from)).collect()
}

/// `a, (b = p)` with p an instance
pub(super) fn last_assigns_instance(items: &[Node], class_of: &ClassOf) -> bool {
	let leading_names =
		items[..items.len().saturating_sub(1)].iter().all(|item| matches!(item.drop_meta(), Node::Symbol(_)));
	leading_names
		&& items.len() > 1
		&& matches!(items.last().map(Node::drop_meta), Some(Node::Key(name, Op::Assign, value)) if matches!(name.drop_meta(), Node::Symbol(_)) && class_of(value).is_some())
}

/// A map literal `{x:1 y:2}` or parsed json `parse_json(t)`: what `as Point` builds an instance from
pub(super) fn is_plain_object(object: &Node) -> bool {
	match object.drop_meta() {
		Node::List(_, Bracket::Curly, _) => true,
		Node::List(items, Bracket::Round, _) => {
			items.first().is_some_and(|word| word.drop_meta().name() == PARSE_JSON_WORD)
		}
		_ => false,
	}
}

/// `int` of `x:int`, `[Point]` of `points:[Point]`, the empty name for an untyped field
pub(super) fn field_type_name(item: &Node) -> String {
	match item.drop_meta() {
		Node::Key(_, Op::Colon, field_type) => match field_type.drop_meta() {
			Node::Type { name, .. } => name.drop_meta().name(),
			Node::List(items, Bracket::Square, _) if items.len() == 1 => format!("[{}]", items[0].drop_meta().name()),
			// a compound unit, `speed: km/h` (card unit-fields)
			unit @ Node::Key(_, Op::Div | Op::Mul, _) => unit.serialize().trim().to_string(),
			other => type_word(other),
		},
		Node::Key(field, Op::Assign, _) => field_type_name(field),
		_ => String::new(),
	}
}

/// The type an annotation names: `int`, and an inline union `int | text` or `Circle | Square` whole, as `int or text`
pub(crate) fn type_word(annotation: &Node) -> String {
	crate::analyzer::union_of_types(annotation, &|word| crate::analyzer::builtin_type_kind(word).is_none())
		.unwrap_or_else(|| annotation.name())
}
