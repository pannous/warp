//! Lazy reads: a table read as its load, a row, a slice or a page of positions, foreign keys resolved to their rows
use super::*;
use crate::lowering::nodes::block;

/// The code with each placeholder as its node and the generated variables named
pub(super) fn generated<'a>(code: &str, placeholders: impl IntoIterator<Item = (&'a str, Node)>) -> Node {
	let names = GENERATED_NAMES.iter().map(|(written, name)| (written.to_string(), symbol(name)));
	// generated after times_phrases ran: `n times "?, "` gets its meaning here
	crate::law::substitute(
		&crate::lowering::times_phrases::lower(crate::warp_parser::parse_generated(code)),
		&names.chain(placeholders.into_iter().map(|(placeholder, node)| (placeholder.to_string(), node))).collect(),
	)
}

/// The placeholders of a table's lazy parts as their names: `table_load` of people is `people·load`
pub(super) fn lazy_names(variable: &str) -> Vec<(&'static str, Node)> {
	LAZY_PARTS.iter().map(|(placeholder, part)| (*placeholder, Node::Symbol(lazy_name(variable, part)))).collect()
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
	LazyReads { tables, own }.read(node)
}

struct LazyReads<'a> {
	tables: &'a BTreeMap<String, Table>,
	own: Option<&'a str>,
}

impl LazyReads<'_> {
	/// The table a node names, unless it is the one whose lazy functions are being defined
	fn table_of(&self, node: &Node) -> Option<String> {
		match node.drop_meta() {
			Node::Symbol(name) if self.tables.contains_key(name) && self.own != Some(name.as_str()) => {
				Some(name.clone())
			}
			_ => None,
		}
	}

	fn read(&self, node: Node) -> Node {
		if let Some(variable) = lazy_definition(&node, self.tables) {
			return node.map_children(|child| with_lazy_reads(child, self.tables, Some(&variable)));
		}
		match node.drop_meta().clone() {
			Node::Symbol(_) => match self.table_of(&node) {
				Some(variable) => called(lazy_name(&variable, "load"), None),
				None => node,
			},
			Node::Key(_, Op::Assign, source) if database_table(&source).is_some() => node,
			Node::Key(word, Op::Colon, _) if word.drop_meta().name() == GLOBAL_WORD => node,
			Node::List(parts, _, _)
				if let [word, listed] = parts.as_slice()
					&& crate::analyzer::is_counting_word(&word.drop_meta().name())
					&& let Some(variable) = self.table_of(listed) =>
			{
				called(lazy_name(&variable, "count"), None)
			}
			// `people#(11..20)`, `people[10:20]` (the parser's `slice(people, 10, 20)`; an open bound is ø)
			Node::List(ref parts, Bracket::Round, _)
				if let [word, listed, from, until] = parts.as_slice()
					&& word.is_symbol(crate::words::SLICE)
					&& let Some(variable) = self.table_of(listed) =>
			{
				self.slice_read(&variable, from, until)
			}
			Node::List(ref parts, _, _)
				if let [for_word, item, in_word, listed, body] = parts.as_slice()
					&& for_word.drop_meta().name() == FOR_WORD
					&& in_word.drop_meta().name() == IN_WORD
					&& matches!(item.drop_meta(), Node::Symbol(_))
					&& let Some(variable) = self.table_of(listed)
					&& body.is_block() =>
			{
				paged_loop(&item.drop_meta().name(), &variable, &self.tables[&variable].class, self.read(body.clone()))
			}
			// the row keeps its class, so `cups#1.size` reads the field, not the builtin word (card orm-size-field)
			Node::Key(left, Op::Hash, position) if let Some(variable) = self.table_of(&left) => {
				let row = called(lazy_name(&variable, "at"), Some(self.read(*position)));
				Node::meta(row, Node::data(crate::lowering::traits::TypedAs(self.tables[&variable].class.clone())))
			}
			Node::Key(left, Op::Dot, right) => self.method_read(left, right),
			Node::Key(target, op, value)
				if (op == Op::Assign || op.is_compound_assign()) && self.table_of(declared(&target)).is_some() =>
			{
				Node::Key(target, op, Box::new(self.read(*value)))
			}
			Node::Type { name, body } => self.class_read(name, *body),
			_ => node.map_children(|child| self.read(child)),
		}
	}

	fn slice_read(&self, variable: &str, from: &Node, until: &Node) -> Node {
		let bound = |bound: &Node, open: Node| match bound.drop_meta() {
			Node::Empty => open,
			_ => self.read(bound.clone()),
		};
		let count = called(lazy_name(variable, "count"), None);
		sliced(variable, bound(from, Node::int(0)), bound(until, count))
	}

	/// `people.count`, `people.add(p)` and the other methods of a table; other methods are of the value read
	fn method_read(&self, left: Box<Node>, right: Box<Node>) -> Node {
		if let Some(variable) = self.table_of(&left) {
			match right.drop_meta() {
				Node::Symbol(word) if crate::analyzer::is_counting_word(word) => {
					return called(lazy_name(&variable, "count"), None);
				}
				Node::List(parts, _, _) => {
					if let Some(read) = self.table_method_read(&variable, parts) {
						return read;
					}
				}
				_ => {}
			}
		}
		match right.drop_meta() {
			Node::Symbol(_) => Node::Key(Box::new(self.read(*left)), Op::Dot, right),
			_ => key(self.read(*left), Op::Dot, self.read(*right)),
		}
	}

	fn table_method_read(&self, variable: &str, parts: &[Node]) -> Option<Node> {
		let word = parts.first()?.drop_meta().name();
		let argument = || self.read(parts[1].clone());
		Some(match (parts.len(), word.as_str()) {
			(3, FOUND_WORD) => {
				let code = format!("{FUNCTION_PLACEHOLDER}({CONDITION_PLACEHOLDER}, {PARAMETERS_PLACEHOLDER})");
				generated(
					&code,
					[
						(FUNCTION_PLACEHOLDER, Node::Symbol(lazy_name(variable, "found"))),
						(CONDITION_PLACEHOLDER, parts[1].clone()),
						(PARAMETERS_PLACEHOLDER, self.read(parts[2].clone())),
					],
				)
			}
			(2, ADD_WORD | REMOVE_WORD) => called(lazy_name(variable, &word), Some(argument())),
			(2, end) if END_WORDS.contains(&end) => end_slice(variable, end, argument()),
			(2, INSTANCE_OF_WORD) => called(lazy_name(variable, "of"), Some(argument())),
			(2, FETCHED_WORD) => called(lazy_name(variable, "fetched"), Some(argument())),
			_ => return None,
		})
	}

	/// A class's own field named like a table is the field
	fn class_read(&self, name: Box<Node>, body: Node) -> Node {
		let fields: Vec<String> =
			crate::class_methods::field_declarations(&body).into_iter().map(|(field, ..)| field).collect();
		let visible: BTreeMap<String, Table> = self
			.tables
			.iter()
			.filter(|(variable, _)| !fields.contains(variable))
			.map(|(variable, table)| (variable.clone(), table.clone()))
			.collect();
		Node::Type { name, body: Box::new(in_method_bodies(body, &|child| with_lazy_reads(child, &visible, self.own))) }
	}
}

/// The rows of a table from the 0-based `from` to the exclusive `until`, read alone until it loads: `people·sliced(10, 20)`
pub(super) fn sliced(variable: &str, from: Node, until: Node) -> Node {
	generated(
		&format!("{SLICED}({FROM_PLACEHOLDER}, {VALUE_PLACEHOLDER})"),
		[(FROM_PLACEHOLDER, from), (VALUE_PLACEHOLDER, until)].into_iter().chain(lazy_names(variable)),
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
	let statements = block([vec![element_read], body.body_statements()].concat());
	let code = format!("{LOOP_END} = {COUNTED}()\nfor {LOOP_POSITION} in 1 to {LOOP_END} {VALUE_PLACEHOLDER}");
	let lowered = generated(&code, names().chain([(VALUE_PLACEHOLDER, statements)]));
	Node::List(lowered.children(), Bracket::None, Separator::Semicolon)
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
	format!("{}.{INSTANCE_OF_WORD}({id})", table.reference(field).unwrap_or_default())
}

/// A foreign key to the table's own class (`boss: Person?`): its rows are read while the table loads, so they are
/// matched in the list read so far rather than in the identity map, whose finding would load the table again
pub(super) fn is_own_reference(table: &Table, field: &str) -> bool {
	table.fields.iter().any(|(name, field_type, _)| name == field && class_of(field_type) == table.class)
}

/// The rows of the table the foreign key `field` points to whose id is the value of `id`: one, or none when dangling
pub(super) fn matching_rows(table: &Table, field: &str, id: &str) -> String {
	let target = table.reference(field).unwrap_or_default();
	format!("[{REFERENCED} for {REFERENCED} in {target} if {REFERENCED}.{ID_FIELD} == {id}]")
}

pub(super) fn implicit_id(table: &Table) -> bool {
	!table.fields.iter().any(|(name, _, _)| name == ID_FIELD)
}
