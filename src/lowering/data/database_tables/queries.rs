//! Queries: `people where age > 7` as SQL where SQL means the same, the rest through query functions; instances restored from rows
use super::*;

/// `people where age > 7 and is_prime(age)` of a table: `people.table·found(condition, parameters)` of the query
/// `SELECT … WHERE "age" > ? AND warp_call(…)`, which with_lazy_reads makes `people·found(…)`: it reads only the rows
/// the query keeps, each the one instance of its row. The table stays named, so a page reading the filter keeps it. What
/// SQL can say stays SQL; any other part is a function of the program the query calls per row.
pub fn queried(
	subject: &Node,
	condition: &Node,
	variables: &HashSet<String>,
	tables: &mut Tables,
) -> Option<Result<Node, Node>> {
	if cfg!(not(feature = "native")) {
		return None; // the browser's tables have no SQL: their rows are the list already
	}
	let table = tables.tables.get(&subject.drop_meta().name())?;
	let first_function = tables.functions.len();
	let mut query =
		Query { table, variables, parameters: vec![], functions: vec![], first_function, mismatches: vec![] };
	let sql = query.sql(condition);
	if let Some(mismatch) = query.mismatches.into_iter().next() {
		return Some(Err(mismatch.into_error()));
	}
	let (parameters, functions) = (query.parameters, query.functions);
	if let Err(error) = crate::errors::diagnostic::report(&effectful_calls(&functions, tables.effects.as_ref())) {
		return Some(Err(error));
	}
	tables.functions.extend(functions);
	let parameters = if parameters.is_empty() {
		crate::warp_parser::parse_generated("[]")
	} else {
		Node::List(parameters, Bracket::Square, Separator::Space)
	};
	let query = Node::List(vec![symbol(FOUND_WORD), Node::Text(sql), parameters], Bracket::None, Separator::Space);
	Some(Ok(key(subject.drop_meta().clone(), Op::Dot, query)))
}

/// `people.table·found(…)`: a table's filter, a list
pub(crate) fn is_filter_query(node: &Node) -> bool {
	matches!(node.drop_meta(), Node::Key(_, Op::Dot, query) if matches!(query.drop_meta(), Node::List(parts, _, _) if parts.first().is_some_and(|word| word.drop_meta().name() == FOUND_WORD)))
}

/// A warning for each function with side effects that a query's functions call: SQLite calls it once per row, and
/// how often or in which order a query runs is its own business (card orm-filter)
pub(super) fn effectful_calls(
	functions: &[Node],
	effects: Option<&crate::analyzer::effects::EffectReport>,
) -> Vec<crate::errors::diagnostic::Diagnostic> {
	let Some(effects) = effects else { return vec![] };
	let mut warned: Vec<String> = vec![];
	let mut warnings = vec![];
	for function in functions {
		function.visit(&mut |node| if let Node::Symbol(name) = node {
			let side_effects: Vec<String> = effects.effects_of(name).into_iter().flat_map(|found| found.iter())
				.filter(|effect| SIDE_EFFECTS.contains(effect)).map(|effect| format!("{effect:?}")).collect();
			if !side_effects.is_empty() && !warned.contains(name) {
				warned.push(name.clone());
				warnings.push(crate::errors::diagnostic::Diagnostic::at(node, format!(
					"{name} has side effects ({}): a table filter runs it once per row inside its query; keep filters pure", side_effects.join(", "))));
			}
		});
	}
	warnings
}

/// The program with the functions its filters' queries call, first
pub fn with_filter_functions(program: Node, tables: Tables) -> Node {
	if tables.functions.is_empty() {
		return program;
	}
	let statements = match program {
		Node::List(statements, Bracket::None, Separator::Semicolon | Separator::Newline) => statements,
		program => vec![program],
	};
	Node::List([tables.functions, statements].concat(), Bracket::None, Separator::Newline)
}

/// One filter's condition as SQL: its `?` parameters and the functions it calls
pub(super) struct Query<'a> {
	pub(super) table: &'a Table,
	pub(super) variables: &'a HashSet<String>,
	pub(super) parameters: Vec<Node>,
	pub(super) functions: Vec<Node>,
	pub(super) first_function: usize,
	/// a unit column compared with another kind of quantity: `it.distance > 5 s`
	pub(super) mismatches: Vec<crate::errors::diagnostic::Diagnostic>,
}

impl Query<'_> {
	pub(super) fn sql(&mut self, node: &Node) -> String {
		self.translated(node).unwrap_or_else(|| self.called(node))
	}

	/// The part as SQL when SQL means the same: comparisons, and/or, arithmetic of numbers, columns, values
	pub(super) fn translated(&mut self, node: &Node) -> Option<String> {
		if let Some(column) = self.column(node) {
			return Some(column);
		}
		if let Some(compared) = self.key_comparison(node) {
			return Some(compared);
		}
		match node.drop_meta() {
			Node::Key(left, op, right) => {
				let operator = match op {
					// IS: ø (NULL) equals ø, as in warp (`it.email == ø` of an optional field)
					Op::Eq => "IS",
					Op::Ne => "IS NOT",
					Op::Lt => "<",
					Op::Gt => ">",
					Op::Le => "<=",
					Op::Ge => ">=",
					Op::And => "AND",
					Op::Or => "OR",
					// warp's division is exact and its + joins texts: only these keep their meaning on numbers
					Op::Add if self.is_numeric(left) && self.is_numeric(right) => "+",
					Op::Sub if self.is_numeric(left) && self.is_numeric(right) => "-",
					Op::Mul if self.is_numeric(left) && self.is_numeric(right) => "*",
					_ => return None,
				};
				let (left_unit, right_unit) = (self.column_unit(left), self.column_unit(right));
				Some(format!(
					"({} {operator} {})",
					self.sql_beside(left, right_unit),
					self.sql_beside(right, left_unit)
				))
			}
			Node::Number(_) | Node::Text(_) | Node::True | Node::False => {
				Some(self.parameter(node.drop_meta().clone()))
			}
			Node::Symbol(name) if self.variables.contains(name) => Some(self.parameter(node.drop_meta().clone())),
			_ => None,
		}
	}

	/// A part compared with a unit column: a constant quantity of its kind is its SI amount, as the column holds it
	/// (`it.distance > 5 km` asks `"distance" > 5000`)
	pub(super) fn sql_beside(&mut self, node: &Node, column_unit: Option<String>) -> String {
		match column_unit.zip(si_quantity(node)) {
			Some((unit, (amount, quantity))) if quantity == unit => self.parameter(crate::node::float(amount)),
			Some((unit, (_, quantity))) => {
				self.mismatches.push(crate::errors::diagnostic::Diagnostic::at(
					node,
					format!("DimensionError: a column of {unit} compared with {quantity}"),
				));
				self.sql(node)
			}
			None => self.sql(node),
		}
	}

	/// The quantity a unit column measures (`m` of `distance: km`)
	pub(super) fn column_unit(&self, node: &Node) -> Option<String> {
		let field = self.field_of_it(node)?;
		let (_, field_type, _) = self.table.fields.iter().find(|(name, _, _)| *name == field)?;
		crate::numbers::units::static_units::unit_type(class_of(field_type)).map(|(quantity, _)| quantity)
	}

	/// `it.age` as the column "age", `it.id` as id
	pub(super) fn column(&self, node: &Node) -> Option<String> {
		let field = self.field_of_it(node)?;
		match field == ID_FIELD {
			true => Some(ID_FIELD.to_string()),
			// a foreign key's column is an id, the field an instance: compared through a query's function
			false => (columns_of(self.table).contains(&field) && self.table.reference(&field).is_none())
				.then(|| quote(&field)),
		}
	}

	/// `it.team == red` of a foreign key team: its column against the id of red's row, 0 for ø
	pub(super) fn key_comparison(&mut self, node: &Node) -> Option<String> {
		let Node::Key(left, op @ (Op::Eq | Op::Ne), right) = node.drop_meta() else {
			return None;
		};
		let field = self.field_of_it(left).filter(|field| self.table.reference(field).is_some())?;
		if right.mentions(crate::words::IMPLICIT_PARAMETER) {
			return None;
		}
		let id = generated(
			&format!("(if {VALUE_PLACEHOLDER} == ø then 0 else {VALUE_PLACEHOLDER}.{ID_FIELD})"),
			[(VALUE_PLACEHOLDER, right.as_ref().clone())],
		);
		let operator = if *op == Op::Eq { "=" } else { "<>" };
		Some(format!("({} {operator} {})", quote(&field), self.parameter(id)))
	}

	pub(super) fn field_of_it(&self, node: &Node) -> Option<String> {
		let Node::Key(element, Op::Dot, field) = node.drop_meta() else {
			return None;
		};
		let field = field.drop_meta();
		(element.drop_meta().name() == crate::words::IMPLICIT_PARAMETER && matches!(field, Node::Symbol(_)))
			.then(|| field.name())
	}

	pub(super) fn is_numeric(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Number(_) => true,
			_ => self.field_of_it(node).is_some_and(|field| {
				field == ID_FIELD
					|| self
						.table
						.fields
						.iter()
						.any(|(name, field_type, _)| *name == field && NUMERIC_TYPES.contains(&field_type.as_str()))
			}),
		}
	}

	pub(super) fn parameter(&mut self, value: Node) -> String {
		self.parameters.push(value);
		"?".to_string()
	}

	/// The part as `warp_call('table·call·N', id, columns…, values…)`: the program's function table·call·N, generated
	/// here, works it out of the row and the filter's values
	pub(super) fn called(&mut self, node: &Node) -> String {
		let name = format!("{FUNCTION}·{}", self.first_function + self.functions.len() + 1);
		let mut values = vec![];
		free_variables(node, self.variables, &mut values);
		let row_size = 1 + columns_of(self.table).len();
		let body = with_arguments(with_si_amounts(node.clone()), self.table, &values, row_size);
		let function = generated(
			&format!("{FUNCTION_PLACEHOLDER}({ARGUMENTS}: any) := {BODY_PLACEHOLDER}"),
			[(FUNCTION_PLACEHOLDER, Node::Symbol(name.clone())), (BODY_PLACEHOLDER, body)],
		);
		self.functions.push(function);
		let row: Vec<String> = std::iter::once(ID_FIELD.to_string())
			.chain(columns_of(self.table).iter().map(|column| quote(column)))
			.collect();
		let values: Vec<String> = values.into_iter().map(|value| self.parameter(Node::Symbol(value))).collect();
		format!("{WARP_CALL}('{name}', {})", [row, values].concat().join(", "))
	}
}

/// A query's function gets a unit column as its SI amount: a constant quantity in it is one too (`5 km` is 5000.0)
pub(super) fn with_si_amounts(node: Node) -> Node {
	match si_quantity(&node) {
		Some((amount, _)) if !matches!(node.drop_meta(), Node::Symbol(_)) => crate::node::float(amount),
		_ => node.map_children(with_si_amounts),
	}
}

/// The variables a part reads, in order, each once (not a name after a dot: a field or method)
pub(super) fn free_variables(node: &Node, variables: &HashSet<String>, found: &mut Vec<String>) {
	match node.drop_meta() {
		Node::Symbol(name) if variables.contains(name) && !found.contains(name) => found.push(name.clone()),
		Node::Key(left, Op::Dot, _) => free_variables(left, variables, found),
		Node::Key(left, _, right) => {
			free_variables(left, variables, found);
			free_variables(right, variables, found);
		}
		Node::List(items, _, _) => items.iter().for_each(|item| free_variables(item, variables, found)),
		_ => {}
	}
}

/// The part in a query's function: `it.age` as its column's argument, `it` as the row's instance, a variable as the
/// filter's value passed after the row
pub(super) fn with_arguments(node: Node, table: &Table, values: &[String], row_size: usize) -> Node {
	let argument = |index: usize| generated(&format!("{ARGUMENTS}#{index}"), []);
	let columns = columns_of(table);
	match node {
		Node::Symbol(name) if name == crate::words::IMPLICIT_PARAMETER => {
			generated(&format!("{}({})", table.class, constructor_arguments(table, ARGUMENTS).join(", ")), [])
		}
		Node::Symbol(name) if values.contains(&name) => {
			argument(row_size + 1 + values.iter().position(|value| *value == name).unwrap_or_default())
		}
		Node::Key(element, Op::Dot, field) if element.drop_meta().name() == crate::words::IMPLICIT_PARAMETER => {
			match field.drop_meta().name() {
				name if name == ID_FIELD => argument(1),
				name if columns.contains(&name) => {
					let index = 2 + columns.iter().position(|column| *column == name).unwrap_or_default();
					match table.reference(&name) {
						Some(_) => generated(&referenced(table, &name, &format!("{ARGUMENTS}#{index}")), []),
						None => argument(index),
					}
				}
				_ => Node::Key(Box::new(with_arguments(*element, table, values, row_size)), Op::Dot, field),
			}
		}
		Node::Key(left, Op::Dot, right) => {
			Node::Key(Box::new(with_arguments(*left, table, values, row_size)), Op::Dot, right)
		}
		other => other.map_children(|child| with_arguments(child, table, values, row_size)),
	}
}

/// The constructor's arguments of an instance from a row `[id, column values…]` held in `row`
pub(super) fn constructor_arguments(table: &Table, row: &str) -> Vec<String> {
	let column = |name: &str| column_cell(table, row, name);
	table
		.fields
		.iter()
		.filter(|(name, _, _)| !table.is_members(name))
		.map(|(name, field_type, _)| match (name == ID_FIELD, table.reference(name)) {
			(true, _) => format!("{row}#1"),
			(false, Some(_)) => referenced(table, name, &column(name)),
			_ if field_type == BOOL_TYPE => format!("({} != 0)", column(name)),
			_ => column(name),
		})
		.chain(implicit_id(table).then(|| format!("{row}#1")))
		.collect()
}

/// The id a foreign key of `instance` stores: its row's, 0 for an optional key holding ø
pub(super) fn key_id(instance: &str, field: &str, optional: bool) -> String {
	match optional {
		true => format!("(if {instance}.{field} == ø then 0 else {instance}.{field}.{ID_FIELD})"),
		false => format!("{instance}.{field}.{ID_FIELD}"),
	}
}

/// Each instance of `list` given the values of its row in `table_rows` again, as constructor_arguments reads them; id 0
/// when its row is gone
pub(super) fn restored_rows(table: &Table, arguments: &[String], list: &str) -> String {
	let fields = table.fields.iter().filter(|(name, _, _)| !table.is_members(name)).map(|(name, _, _)| name);
	let restored: String = fields
		.zip(arguments)
		.filter(|(name, _)| *name != ID_FIELD)
		.map(|(name, argument)| format!("{MADE}.{name} = {argument}\n"))
		.collect();
	format!(
		"for {MADE} in {list} {{
if {MADE}.{ID_FIELD} in {KNOWN} {{
{ROW} = {KNOWN}[{MADE}.{ID_FIELD}]
{restored}}} else {{ {MADE}.{ID_FIELD} = 0 }}
}}"
	)
}

/// The cell of column `name` in `row`, a row being `[id, column values…]`
pub(super) fn column_cell(table: &Table, row: &str, name: &str) -> String {
	format!("{row}#{}", 2 + columns_of(table).iter().position(|column| column == name).unwrap_or_default())
}
