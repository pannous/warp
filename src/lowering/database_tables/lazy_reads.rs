//! Lazy reads: a table read as its load, a row, a slice or a page of positions, foreign keys resolved to their rows
use super::*;

/// The code with each placeholder as its node and the generated variables named
pub(super) fn generated<'a>(code: &str, placeholders: impl IntoIterator<Item = (&'a str, Node)>) -> Node {
	let names = GENERATED_NAMES
		.iter()
		.map(|(written, name)| (written.to_string(), symbol(name)));
	// generated after times_phrases ran: `n times "?, "` gets its meaning here
	crate::law::substitute(
		&crate::lowering::times_phrases::lower(parse(code)),
		&names
			.chain(
				placeholders
					.into_iter()
					.map(|(placeholder, node)| (placeholder.to_string(), node)),
			)
			.collect(),
	)
}

/// The placeholders of a table's lazy parts as their names: `table_load` of people is `people·load`
pub(super) fn lazy_names(variable: &str) -> Vec<(&'static str, Node)> {
	LAZY_PARTS
		.iter()
		.map(|(placeholder, part)| (*placeholder, Node::Symbol(lazy_name(variable, part))))
		.collect()
}

pub(super) fn lazy_name(variable: &str, part: &str) -> String {
	format!("{variable}·{part}")
}

/// The call `name()` or `name(argument)`
pub(super) fn called(name: String, argument: Option<Node>) -> Node {
	let code = match argument {
		Some(_) => format!("{FUNCTION_PLACEHOLDER}({VALUE_PLACEHOLDER})"),
		None => format!("{FUNCTION_PLACEHOLDER}()"),
	};
	generated(
		&code,
		[(FUNCTION_PLACEHOLDER, Node::Symbol(name))]
			.into_iter()
			.chain(argument.map(|value| (VALUE_PLACEHOLDER, value))),
	)
}

/// Reads of a table's list as its load (`people` → `people·load()`), `count(people)` as its count, `people.add(p)` as
/// its add and `people#i` as its element, which load no rows; a table's own lazy functions keep its list (`own`). Registrations, `global people`,
/// assigned lists and field names (`x.people`) stay
pub(super) fn with_lazy_reads(node: Node, tables: &BTreeMap<String, Table>, own: Option<&str>) -> Node {
	let table_of = |node: &Node| match node.drop_meta() {
		Node::Symbol(name) if tables.contains_key(name) && own != Some(name.as_str()) => Some(name.clone()),
		_ => None,
	};
	let rewrite = |node: Node| with_lazy_reads(node, tables, own);
	if let Some(variable) = lazy_definition(&node, tables) {
		return node.map_children(|child| with_lazy_reads(child, tables, Some(&variable)));
	}
	match node.drop_meta().clone() {
		Node::Symbol(_) => match table_of(&node) {
			Some(variable) => called(lazy_name(&variable, "load"), None),
			None => node,
		},
		Node::Key(_, Op::Assign, source) if database_table(&source).is_some() => node,
		Node::Key(word, Op::Colon, _) if word.drop_meta().name() == GLOBAL_WORD => node,
		Node::List(parts, _, _)
			if parts.len() == 2
				&& crate::analyzer::is_counting_word(&parts[0].drop_meta().name())
				&& table_of(&parts[1]).is_some() =>
		{
			called(lazy_name(&table_of(&parts[1]).unwrap_or_default(), "count"), None)
		}
		// `people#(11..20)`, `people[10:20]` (the parser's `slice(people, 10, 20)`; an open bound is ø)
		Node::List(parts, Bracket::Round, _)
			if parts.len() == 4 && parts[0].is_symbol(crate::words::SLICE) && table_of(&parts[1]).is_some() =>
		{
			let variable = table_of(&parts[1]).unwrap_or_default();
			let bound = |bound: &Node, open: Node| match bound.drop_meta() {
				Node::Empty => open,
				_ => rewrite(bound.clone()),
			};
			let count = called(lazy_name(&variable, "count"), None);
			sliced(&variable, bound(&parts[2], Node::int(0)), bound(&parts[3], count))
		}
		Node::List(parts, _, _)
			if parts.len() == 5
				&& parts[0].drop_meta().name() == FOR_WORD
				&& parts[2].drop_meta().name() == IN_WORD
				&& matches!(parts[1].drop_meta(), Node::Symbol(_))
				&& table_of(&parts[3]).is_some()
				&& matches!(parts[4].drop_meta(), Node::List(_, Bracket::Curly, _)) =>
		{
			let variable = table_of(&parts[3]).unwrap_or_default();
			paged_loop(
				&parts[1].drop_meta().name(),
				&variable,
				&tables[&variable].class,
				rewrite(parts[4].clone()),
			)
		}
		// the row keeps its class, so `cups#1.size` reads the field, not the builtin word (card orm-size-field)
		Node::Key(left, Op::Hash, position) if table_of(&left).is_some() => {
			let variable = table_of(&left).unwrap_or_default();
			let row = called(lazy_name(&variable, "at"), Some(rewrite(*position)));
			Node::meta(
				row,
				Node::data(crate::lowering::traits::TypedAs(tables[&variable].class.clone())),
			)
		}
		Node::Key(left, Op::Dot, right) => match (table_of(&left), right.drop_meta()) {
			(Some(variable), Node::Symbol(word)) if crate::analyzer::is_counting_word(word) => {
				called(lazy_name(&variable, "count"), None)
			}
			(Some(variable), Node::List(parts, _, _))
				if parts.len() == 3 && parts[0].drop_meta().name() == FOUND_WORD =>
			{
				let code = format!("{FUNCTION_PLACEHOLDER}({CONDITION_PLACEHOLDER}, {PARAMETERS_PLACEHOLDER})");
				generated(
					&code,
					[
						(FUNCTION_PLACEHOLDER, Node::Symbol(lazy_name(&variable, "found"))),
						(CONDITION_PLACEHOLDER, parts[1].clone()),
						(PARAMETERS_PLACEHOLDER, rewrite(parts[2].clone())),
					],
				)
			}
			(Some(variable), Node::List(parts, _, _))
				if parts.len() == 2 && [ADD_WORD, REMOVE_WORD].contains(&parts[0].drop_meta().name().as_str()) =>
			{
				called(
					lazy_name(&variable, &parts[0].drop_meta().name()),
					Some(rewrite(parts[1].clone())),
				)
			}
			(Some(variable), Node::List(parts, _, _))
				if parts.len() == 2 && END_WORDS.contains(&parts[0].drop_meta().name().as_str()) =>
			{
				end_slice(&variable, &parts[0].drop_meta().name(), rewrite(parts[1].clone()))
			}
			(Some(variable), Node::List(parts, _, _))
				if parts.len() == 2 && parts[0].drop_meta().name() == INSTANCE_OF_WORD =>
			{
				called(lazy_name(&variable, "of"), Some(rewrite(parts[1].clone())))
			}
			(Some(variable), Node::List(parts, _, _))
				if parts.len() == 2 && parts[0].drop_meta().name() == FETCHED_WORD =>
			{
				called(lazy_name(&variable, "fetched"), Some(rewrite(parts[1].clone())))
			}
			(_, Node::Symbol(_)) => Node::Key(Box::new(rewrite(*left)), Op::Dot, right),
			_ => key(rewrite(*left), Op::Dot, rewrite(*right)),
		},
		Node::Key(target, op, value)
			if (op == Op::Assign || op.is_compound_assign()) && table_of(declared(&target)).is_some() =>
		{
			Node::Key(target, op, Box::new(rewrite(*value)))
		}
		// a class's own field named like a table is the field
		Node::Type { name, body } => {
			let fields: Vec<String> = crate::class_methods::field_declarations(&body)
				.into_iter()
				.map(|(field, ..)| field)
				.collect();
			let visible: BTreeMap<String, Table> = tables
				.iter()
				.filter(|(variable, _)| !fields.contains(variable))
				.map(|(variable, table)| (variable.clone(), table.clone()))
				.collect();
			Node::Type {
				name,
				body: Box::new(in_method_bodies(*body, &|child| with_lazy_reads(child, &visible, own))),
			}
		}
		_ => node.map_children(rewrite),
	}
}

/// The rows of a table from the 0-based `from` to the exclusive `until`, read alone until it loads: `people·sliced(10, 20)`
pub(super) fn sliced(variable: &str, from: Node, until: Node) -> Node {
	generated(
		&format!("{SLICED}({FROM_PLACEHOLDER}, {VALUE_PLACEHOLDER})"),
		[(FROM_PLACEHOLDER, from), (VALUE_PLACEHOLDER, until)]
			.into_iter()
			.chain(lazy_names(variable)),
	)
}

/// `first n of people` (`people.first(n)`) and `last n of people` as slices
pub(super) fn end_slice(variable: &str, word: &str, size: Node) -> Node {
	let count = || called(lazy_name(variable, "count"), None);
	match word == END_WORDS[0] {
		true => sliced(variable, Node::int(0), size),
		false => sliced(variable, key(count(), Op::Sub, size), count()),
	}
}

/// A class's methods read its tables as functions do; its fields stay, also one named like a table (map_children would
/// read a field declaration `people: int` as a read of the table)
pub(super) fn in_method_bodies(node: Node, rewrite: &impl Fn(Node) -> Node) -> Node {
	match node {
		Node::Key(head, Op::Define, body) => Node::Key(head, Op::Define, Box::new(rewrite(*body))),
		other => other.map_children(|child| in_method_bodies(child, rewrite)),
	}
}

/// `for p in people body` over an unloaded table walks positions up to its count, each element read from a page
/// (`people·streamed`); the count is taken once, so rows the body adds are not walked
pub(super) fn paged_loop(element: &str, variable: &str, class: &str, body: Node) -> Node {
	let names = || {
		[(LOOP_END, "end"), (LOOP_POSITION, "position")]
			.map(|(placeholder, part)| (placeholder, Node::Symbol(format!("{element}·{part}"))))
			.into_iter()
			.chain(lazy_names(variable))
	};
	// the body's statements follow the element's in one block: a nested block of expressions would read as a list
	let element_read = generated(&format!("{element}: {class} = {STREAMED}({LOOP_POSITION})"), names());
	let statements = Node::List(
		[vec![element_read], body_statements(body)].concat(),
		Bracket::Curly,
		Separator::Semicolon,
	);
	let code = format!("{LOOP_END} = {COUNTED}()\nfor {LOOP_POSITION} in 1 to {LOOP_END} {VALUE_PLACEHOLDER}");
	let lowered = generated(&code, names().chain([(VALUE_PLACEHOLDER, statements)]));
	Node::List(lowered.children(), Bracket::None, Separator::Semicolon)
}

/// A block's statements: `{print p.name}` is one statement, its words a call, not the statements `print` and `p.name`
pub(super) fn body_statements(body: Node) -> Vec<Node> {
	match body.drop_meta() {
		Node::List(items, Bracket::Curly, Separator::Space) if items.len() > 1 => {
			vec![Node::List(items.clone(), Bracket::None, Separator::Space)]
		}
		_ => body.children(),
	}
}

/// The table of a generated element read, `people·at(i)` or `people·streamed(i)`: static units knows its class
pub(crate) fn element_table(node: &Node) -> Option<String> {
	lazy_part_table(node, &[ELEMENT, STREAMED])
}

/// The table of a generated read of its whole list, `people·load()` where the program reads `people`, or of a slice of it
pub(crate) fn loaded_table(node: &Node) -> Option<String> {
	lazy_part_table(node, &[LOAD, SLICED])
}

/// The table of a call of one of its lazy parts `placeholders`
pub(super) fn lazy_part_table(node: &Node, placeholders: &[&str]) -> Option<String> {
	let Node::List(items, Bracket::Round, _) = node.drop_meta() else {
		return None;
	};
	let name = items.first()?.drop_meta().name();
	let (table, part) = name.rsplit_once('·')?;
	LAZY_PARTS
		.iter()
		.any(|(placeholder, lazy_part)| placeholders.contains(placeholder) && *lazy_part == part)
		.then(|| table.to_string())
}

/// The variable an assignment's target names: `people` of `people: [Person]`
pub(super) fn declared(target: &Node) -> &Node {
	match target.drop_meta() {
		Node::Key(variable, Op::Colon, _) => variable,
		_ => target,
	}
}

/// `people·load() := …` and the other lazy functions of a table: its variable
pub(super) fn lazy_definition(node: &Node, tables: &BTreeMap<String, Table>) -> Option<String> {
	let Node::Key(head, Op::Define, _) = node.drop_meta() else {
		return None;
	};
	let name = match head.drop_meta() {
		Node::List(parts, _, _) => parts.first()?.drop_meta().name(),
		other => other.name(),
	};
	let (variable, part) = name.rsplit_once('·')?;
	(tables.contains_key(variable) && LAZY_PARTS.iter().any(|(_, lazy_part)| *lazy_part == part))
		.then(|| variable.to_string())
}

/// The table's columns besides id, in field order
pub(super) fn columns_of(table: &Table) -> Vec<String> {
	column_fields(table).into_iter().map(|(name, _, _)| name).collect()
}

/// The fields kept in columns besides id (not one-to-many lists), a foreign key as the int id of its row
pub(super) fn column_fields(table: &Table) -> Vec<(String, String, Option<Node>)> {
	table
		.fields
		.iter()
		.filter(|(name, _, _)| name != ID_FIELD && !table.is_members(name))
		.map(|(name, field_type, default)| match table.reference(name) {
			Some(_) => (name.clone(), NUMERIC_TYPES[0].to_string(), None),
			None => (name.clone(), field_type.clone(), default.clone()),
		})
		.collect()
}

/// The row whose id is the value of `id` (the instance the foreign key `field` points to); ø for an optional key
/// without its row
pub(super) fn referenced(table: &Table, field: &str, id: &str) -> String {
	if !is_own_reference(table, field) {
		return referenced_instance(table, field, id);
	}
	let matches = matching_rows(table, field, id);
	match table.is_optional(field) {
		true => format!("({MATCHES} = {matches}; if {MATCHES} then {MATCHES}#1 else ø)"),
		false => format!("{matches}#1"),
	}
}

/// The instance of the row of the other table whose id is the value of `id`, found in its identity map, ø when none
pub(super) fn referenced_instance(table: &Table, field: &str, id: &str) -> String {
	format!(
		"{}.{INSTANCE_OF_WORD}({id})",
		table.reference(field).unwrap_or_default()
	)
}

/// A foreign key to the table's own class (`boss: Person?`): its rows are read while the table loads, so they are
/// matched in the list read so far rather than in the identity map, whose finding would load the table again
pub(super) fn is_own_reference(table: &Table, field: &str) -> bool {
	table
		.fields
		.iter()
		.any(|(name, field_type, _)| name == field && class_of(field_type) == table.class)
}

/// The rows of the table the foreign key `field` points to whose id is the value of `id`: one, or none when dangling
pub(super) fn matching_rows(table: &Table, field: &str, id: &str) -> String {
	let target = table.reference(field).unwrap_or_default();
	format!("[{REFERENCED} for {REFERENCED} in {target} if {REFERENCED}.{ID_FIELD} == {id}]")
}

pub(super) fn implicit_id(table: &Table) -> bool {
	!table.fields.iter().any(|(name, _, _)| name == ID_FIELD)
}
