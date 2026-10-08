//! Plain classes as database tables (card orm, notes/orm.md step 1): `people: [Person] = database.people` registers the
//! table people for the class Person. The list starts as the table's rows (`std_io("table", "open", …)` creates the
//! table or migrates it to the class's fields), `people.add(p)` inserts p and gives it its row's id, and a field change
//! `p.age += 1` of an instance with a row is written through (`std_io("table", "update", …)`). Natively the tables live
//! in `<program>.database.sqlite` (in memory for code without a file, database.rs); the browser keeps them in IndexedDB
//! (web/playground/host-files.js), where a filter stays the list comprehension over the rows.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;
use std::collections::{HashMap, HashSet};

const DATABASE_WORDS: [&str; 2] = ["database", "indexedDB"];
/// `app.warp` keeps its tables in `app.database.sqlite`; inline code's tables are this name alone (in memory)
pub const TABLES_FILE: &str = "database.sqlite";
/// The row id every registered class gets, unless it declares one
const ID_FIELD: &str = "id";
const ADD_WORD: &str = "add";
/// `save p` writes every column of p's row (field changes are written through already, so it changes nothing then)
const SAVE_WORD: &str = "save";
/// the generated code's variables, written as these placeholders (`·` would parse as a product)
const ROW: &str = "table_row";
const ADDED: &str = "table_added";
/// a foreign key's row, and the row of a one-to-many list with one of its members
const REFERENCED: &str = "table_referenced";
const OWNER: &str = "table_owner";
const MEMBER: &str = "table_member";
const SAVED: &str = "table_saved";
const GENERATED_NAMES: [(&str, &str); 7] = [(ROW, "table·row"), (ADDED, "table·added"), (ARGUMENTS, "table·arguments"),
	(REFERENCED, "table·referenced"), (OWNER, "table·owner"), (MEMBER, "table·member"), (SAVED, "table·saved")];
const SCHEMA_PLACEHOLDER: &str = "table_schema";
const VALUE_PLACEHOLDER: &str = "table_value";
/// A filter's query (notes/orm.md step 2): the ids it keeps, its SQL condition and parameters, the functions it calls
const IDS_PLACEHOLDER: &str = "table_ids";
const CONDITION_PLACEHOLDER: &str = "table_condition";
const PARAMETERS_PLACEHOLDER: &str = "table_parameters";
const FUNCTION_PLACEHOLDER: &str = "table_function";
const BODY_PLACEHOLDER: &str = "table_body";
/// The one parameter of a query's function: [id, columns…, the filter's values…] of a row
const ARGUMENTS: &str = "table_arguments";
const IDS: &str = "table·ids";
const FUNCTION: &str = "table·call";
/// The SQL function a query calls a warp function through (database.rs)
const WARP_CALL: &str = "warp_call";
const NUMERIC_TYPES: [&str; 4] = ["int", "float", "number", "real"];
/// What a filter's function should not do: not being sure to end (Div) or allocating is fine
const SIDE_EFFECTS: [crate::effects::Effect; 5] = [crate::effects::Effect::State, crate::effects::Effect::IO, crate::effects::Effect::FFI, crate::effects::Effect::Async, crate::effects::Effect::Eval];

/// A registered table: the list variable holding it and its class's fields (name, type, default)
struct Table {
	name: String,
	class: String,
	fields: Vec<(String, String, Option<Node>)>,
	/// fields holding an instance of another table's class, a foreign key: the field and that table's variable
	references: Vec<(String, String)>,
	/// list fields of another table's class, one-to-many: not a column, the other table's rows pointing back
	members: Vec<Members>,
}

/// `members: [Person]` of Team: the rows of `table` (people) whose field `back` (team) is the team
struct Members {
	field: String,
	table: String,
	back: Option<String>,
}

impl Table {
	fn reference(&self, field: &str) -> Option<&str> {
		self.references.iter().find(|(name, _)| name == field).map(|(_, table)| table.as_str())
	}

	fn is_members(&self, field: &str) -> bool {
		self.members.iter().any(|members| members.field == field)
	}
}

pub fn lower(program: Node) -> Node {
	let program = match crate::warp_parser::mentions(&program, crate::stored_values::STORED_WORD) {
		true => {
			let classes = class_bodies(&program);
			with_stored_tables(program, &classes)
		}
		false => program,
	};
	if !DATABASE_WORDS.iter().any(|word| crate::warp_parser::mentions(&program, word)) {
		return program;
	}
	let tables = registrations(&program);
	if tables.is_empty() {
		return program;
	}
	let registered: Vec<&str> = tables.values().map(|table| table.class.as_str()).collect();
	with_tables(with_row_ids(program, &registered), &tables, &tables_file(), &mut vec![])
}

/// The program's registered tables by their list variable
fn registrations(program: &Node) -> HashMap<String, Table> {
	let classes = class_bodies(program);
	let mut tables = HashMap::new();
	program.visit(&mut |part| if let Some((variable, table)) = registration(part, &classes) {
		tables.insert(variable, table);
	});
	with_relations(tables)
}

/// Each table's fields of another table's class (foreign keys) and lists of one (one-to-many, pointing back)
fn with_relations(mut tables: HashMap<String, Table>) -> HashMap<String, Table> {
	let variable_of: HashMap<String, String> = tables.iter().map(|(variable, table)| (table.class.clone(), variable.clone())).collect();
	let fields_of: HashMap<String, Vec<(String, String)>> = tables.values()
		.map(|table| (table.class.clone(), table.fields.iter().map(|(name, field_type, _)| (name.clone(), field_type.clone())).collect())).collect();
	for table in tables.values_mut() {
		for (field, field_type, _) in &table.fields {
			if let Some(target) = variable_of.get(field_type) {
				table.references.push((field.clone(), target.clone()));
			}
			let element = field_type.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or_default();
			if let Some(target) = variable_of.get(element) {
				let back = fields_of[element].iter().find(|(_, back_type)| *back_type == table.class).map(|(name, _)| name.clone());
				table.members.push(Members { field: field.clone(), table: target.clone(), back });
			}
		}
	}
	tables
}

fn tables_file() -> String {
	crate::modules::program_file().map_or_else(|| TABLES_FILE.to_string(), |file| file.with_extension(TABLES_FILE).to_string_lossy().into_owned())
}

/// The tables a program registers, both forms, before database_tables::lower; and the functions its filters' queries call
pub struct Tables {
	tables: HashMap<String, Table>,
	functions: Vec<Node>,
	/// the effects of the program's functions, which a filter's query should not have
	effects: Option<crate::effects::EffectReport>,
}

pub fn registered(program: &Node) -> Tables {
	let registers = crate::warp_parser::mentions(program, crate::stored_values::STORED_WORD) || DATABASE_WORDS.iter().any(|word| crate::warp_parser::mentions(program, word));
	let tables = match registers {
		true => registrations(&with_stored_tables(program.clone(), &class_bodies(program))),
		false => HashMap::new(),
	};
	let effects = (!tables.is_empty()).then(|| crate::effects::EffectReport::of(program));
	Tables { tables, functions: vec![], effects }
}

/// `people where age > 7 and is_prime(age)` of a table: the query `SELECT id … WHERE "age" > ? AND warp_call(…)` as a
/// binding of the ids it keeps, and the condition keeping the list's instances of those ids, so a filtered row is the
/// same instance. What SQL can say stays SQL; any other part is a function of the program the query calls per row.
pub fn queried(subject: &Node, condition: &Node, variables: &HashSet<String>, tables: &mut Tables) -> Option<Result<(Node, Node), Node>> {
	if cfg!(not(feature = "native")) {
		return None; // the browser's tables have no SQL: their rows are the list already
	}
	let table = tables.tables.get(&subject.drop_meta().name())?;
	let first_function = tables.functions.len();
	let mut query = Query { table, variables, parameters: vec![], functions: vec![], first_function };
	let sql = query.sql(condition);
	let (parameters, functions) = (query.parameters, query.functions);
	if let Err(error) = crate::diagnostic::report(&effectful_calls(&functions, tables.effects.as_ref())) {
		return Some(Err(error));
	}
	tables.functions.extend(functions);
	let ids = format!("{IDS}·{}", first_function + 1);
	let code = format!("{IDS_PLACEHOLDER} = std_io(\"table\", \"select\", [{name:?}, {CONDITION_PLACEHOLDER}, {PARAMETERS_PLACEHOLDER}, {file:?}])",
		name = table.name, file = tables_file());
	let binding = generated(&code, [
		(IDS_PLACEHOLDER, Node::Symbol(ids.clone())),
		(CONDITION_PLACEHOLDER, Node::Text(sql)),
		(PARAMETERS_PLACEHOLDER, Node::List(parameters, Bracket::Square, Separator::Space)),
	]);
	let kept = generated(&format!("{}.{ID_FIELD} in {IDS_PLACEHOLDER}", crate::lambdas::IMPLICIT_PARAMETER), [(IDS_PLACEHOLDER, Node::Symbol(ids))]);
	Some(Ok((binding, kept)))
}

/// A warning for each function with side effects that a query's functions call: SQLite calls it once per row, and
/// how often or in which order a query runs is its own business (card orm-filter)
fn effectful_calls(functions: &[Node], effects: Option<&crate::effects::EffectReport>) -> Vec<crate::diagnostic::Diagnostic> {
	let Some(effects) = effects else { return vec![] };
	let mut warned: Vec<String> = vec![];
	let mut warnings = vec![];
	for function in functions {
		function.visit(&mut |node| if let Node::Symbol(name) = node {
			let side_effects: Vec<String> = effects.effects_of(name).into_iter().flat_map(|found| found.iter())
				.filter(|effect| SIDE_EFFECTS.contains(effect)).map(|effect| format!("{effect:?}")).collect();
			if !side_effects.is_empty() && !warned.contains(name) {
				warned.push(name.clone());
				warnings.push(crate::diagnostic::Diagnostic::at(node, format!(
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
struct Query<'a> {
	table: &'a Table,
	variables: &'a HashSet<String>,
	parameters: Vec<Node>,
	functions: Vec<Node>,
	first_function: usize,
}

impl Query<'_> {
	fn sql(&mut self, node: &Node) -> String {
		self.translated(node).unwrap_or_else(|| self.called(node))
	}

	/// The part as SQL when SQL means the same: comparisons, and/or, arithmetic of numbers, columns, values
	fn translated(&mut self, node: &Node) -> Option<String> {
		if let Some(column) = self.column(node) {
			return Some(column);
		}
		match node.drop_meta() {
			Node::Key(left, op, right) => {
				let operator = match op {
					Op::Eq => "=",
					Op::Ne => "<>",
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
				Some(format!("({} {operator} {})", self.sql(left), self.sql(right)))
			}
			Node::Number(_) | Node::Text(_) | Node::True | Node::False => Some(self.parameter(node.drop_meta().clone())),
			Node::Symbol(name) if self.variables.contains(name) => Some(self.parameter(node.drop_meta().clone())),
			_ => None,
		}
	}

	/// `it.age` as the column "age", `it.id` as id
	fn column(&self, node: &Node) -> Option<String> {
		let field = self.field_of_it(node)?;
		match field == ID_FIELD {
			true => Some(ID_FIELD.to_string()),
			// a foreign key's column is an id, the field an instance: compared through a query's function
			false => (columns_of(self.table).contains(&field) && self.table.reference(&field).is_none()).then(|| quote(&field)),
		}
	}

	fn field_of_it(&self, node: &Node) -> Option<String> {
		let Node::Key(element, Op::Dot, field) = node.drop_meta() else { return None };
		let field = field.drop_meta();
		(element.drop_meta().name() == crate::lambdas::IMPLICIT_PARAMETER && matches!(field, Node::Symbol(_))).then(|| field.name())
	}

	fn is_numeric(&self, node: &Node) -> bool {
		match node.drop_meta() {
			Node::Number(_) => true,
			_ => self.field_of_it(node).is_some_and(|field| field == ID_FIELD || self.table.fields.iter().any(|(name, field_type, _)| *name == field && NUMERIC_TYPES.contains(&field_type.as_str()))),
		}
	}

	fn parameter(&mut self, value: Node) -> String {
		self.parameters.push(value);
		"?".to_string()
	}

	/// The part as `warp_call('table·call·N', id, columns…, values…)`: the program's function table·call·N, generated
	/// here, works it out of the row and the filter's values
	fn called(&mut self, node: &Node) -> String {
		let name = format!("{FUNCTION}·{}", self.first_function + self.functions.len() + 1);
		let mut values = vec![];
		free_variables(node, self.variables, &mut values);
		let row_size = 1 + columns_of(self.table).len();
		let body = with_arguments(node.clone(), self.table, &values, row_size);
		let function = generated(&format!("{FUNCTION_PLACEHOLDER}({ARGUMENTS}: any) := {BODY_PLACEHOLDER}"), [
			(FUNCTION_PLACEHOLDER, Node::Symbol(name.clone())),
			(BODY_PLACEHOLDER, body),
		]);
		self.functions.push(function);
		let row: Vec<String> = std::iter::once(ID_FIELD.to_string()).chain(columns_of(self.table).iter().map(|column| quote(column))).collect();
		let values: Vec<String> = values.into_iter().map(|value| self.parameter(Node::Symbol(value))).collect();
		format!("{WARP_CALL}('{name}', {})", [row, values].concat().join(", "))
	}
}

/// The variables a part reads, in order, each once (not a name after a dot: a field or method)
fn free_variables(node: &Node, variables: &HashSet<String>, found: &mut Vec<String>) {
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
fn with_arguments(node: Node, table: &Table, values: &[String], row_size: usize) -> Node {
	let argument = |index: usize| generated(&format!("{ARGUMENTS}#{index}"), []);
	let columns = columns_of(table);
	match node {
		Node::Symbol(name) if name == crate::lambdas::IMPLICIT_PARAMETER => {
			generated(&format!("{}({})", table.class, constructor_arguments(table, ARGUMENTS).join(", ")), [])
		}
		Node::Symbol(name) if values.contains(&name) => argument(row_size + 1 + values.iter().position(|value| *value == name).unwrap_or_default()),
		Node::Key(element, Op::Dot, field) if element.drop_meta().name() == crate::lambdas::IMPLICIT_PARAMETER => match field.drop_meta().name() {
			name if name == ID_FIELD => argument(1),
			name if columns.contains(&name) => {
				let index = 2 + columns.iter().position(|column| *column == name).unwrap_or_default();
				match table.reference(&name) {
					Some(target) => generated(&referenced(target, &format!("{ARGUMENTS}#{index}")), []),
					None => argument(index),
				}
			}
			_ => Node::Key(Box::new(with_arguments(*element, table, values, row_size)), Op::Dot, field),
		},
		Node::Key(left, Op::Dot, right) => Node::Key(Box::new(with_arguments(*left, table, values, row_size)), Op::Dot, right),
		other => other.map_children(|child| with_arguments(child, table, values, row_size)),
	}
}

/// The constructor's arguments of an instance from a row `[id, column values…]` held in `row`
fn constructor_arguments(table: &Table, row: &str) -> Vec<String> {
	let columns = columns_of(table);
	let column = |name: &str| format!("{row}#{}", 2 + columns.iter().position(|column| column == name).unwrap_or_default());
	table.fields.iter().map(|(name, _, _)| match (name == ID_FIELD, table.reference(name)) {
		(true, _) => format!("{row}#1"),
		(false, Some(target)) => referenced(target, &column(name)),
		// one-to-many: filled once the table pointing back is open (members_filled)
		_ if table.is_members(name) => "[]".to_string(),
		_ => column(name),
	}).chain(implicit_id(table).then(|| format!("{row}#1"))).collect()
}

/// "name" as an SQL identifier
fn quote(name: &str) -> String {
	format!("\"{}\"", name.replace('"', "\"\""))
}

/// `stored people: [Person]`, the short form of `people: [Person] = database.people` (user, 2026-10-08)
fn with_stored_tables(node: Node, classes: &HashMap<String, Node>) -> Node {
	if let Node::List(items, _, _) = node.drop_meta() {
		if let [word, declaration] = items.as_slice() {
			if let Node::Key(variable, Op::Colon, list_type) = declaration.drop_meta() {
				let class = match list_type.drop_meta() {
					Node::List(element, Bracket::Square, _) if element.len() == 1 => element[0].drop_meta().name(),
					_ => String::new(),
				};
				if word.drop_meta().name() == crate::stored_values::STORED_WORD && classes.contains_key(&class) {
					let table = Node::Key(Box::new(Node::Symbol(DATABASE_WORDS[0].to_string())), Op::Dot, Box::new(variable.drop_meta().clone()));
					return Node::Key(Box::new(declaration.drop_meta().clone()), Op::Assign, Box::new(table));
				}
			}
		}
	}
	node.map_children(|child| with_stored_tables(child, classes))
}

/// Each class's body by name
pub fn class_bodies(program: &Node) -> HashMap<String, Node> {
	let mut bodies = HashMap::new();
	program.visit(&mut |part| if let Node::Type { name, body } = part {
		bodies.insert(name.drop_meta().name(), body.as_ref().clone());
	});
	bodies
}

/// `people: [Person] = database.people` of a class of the program: the variable and its table
fn registration(statement: &Node, classes: &HashMap<String, Node>) -> Option<(String, Table)> {
	let Node::Key(target, Op::Assign, source) = statement.drop_meta() else { return None };
	let Node::Key(variable, Op::Colon, list_type) = target.drop_meta() else { return None };
	let Node::List(element, Bracket::Square, _) = list_type.drop_meta() else { return None };
	let [class] = element.as_slice() else { return None };
	let name = database_table(source)?;
	let class = class.drop_meta().name();
	let fields = crate::class_methods::field_declarations(classes.get(&class)?);
	Some((variable.drop_meta().name(), Table { name, class, fields, references: vec![], members: vec![] }))
}

/// The table `database.t` names
fn database_table(source: &Node) -> Option<String> {
	let Node::Key(store, Op::Dot, table) = source.drop_meta() else { return None };
	DATABASE_WORDS.contains(&store.drop_meta().name().as_str()).then(|| table.drop_meta().name())
}

/// Each registered class with the field `id: int = 0` unless it has one: 0 until the instance has a row
fn with_row_ids(node: Node, registered: &[&str]) -> Node {
	match node {
		Node::Type { name, body } if registered.contains(&name.drop_meta().name().as_str()) && !has_id(&body) => {
			let id = parse(&format!("{ID_FIELD}: int = 0"));
			let items = match *body {
				Node::List(mut items, Bracket::Curly, separator @ (Separator::Semicolon | Separator::Newline)) => {
					items.push(id);
					Node::List(items, Bracket::Curly, separator)
				}
				Node::List(mut items, Bracket::Curly, _) if items.len() == 1 => {
					items.push(id);
					Node::List(items, Bracket::Curly, Separator::Semicolon)
				}
				// `{int x}`: one field of words
				Node::List(words, Bracket::Curly, separator) => Node::List(vec![Node::List(words, Bracket::None, separator), id], Bracket::Curly, Separator::Semicolon),
				body => Node::List(vec![body, id], Bracket::Curly, Separator::Semicolon),
			};
			Node::Type { name, body: Box::new(items) }
		}
		other => other.map_children(|child| with_row_ids(child, registered)),
	}
}

fn has_id(body: &Node) -> bool {
	crate::class_methods::field_declarations(body).iter().any(|(name, _, _)| name == ID_FIELD)
}

/// The registrations, inserts and field changes of the tables as their table calls, in every block; `open` the tables
/// registered so far, as a foreign key reads the rows of its table
fn with_tables(node: Node, tables: &HashMap<String, Table>, file: &str, open: &mut Vec<String>) -> Node {
	match node {
		Node::List(statements, bracket, separator @ (Separator::Semicolon | Separator::Newline)) => {
			let statements = statements.into_iter().flat_map(|statement| table_statements(statement, tables, file, open)).collect();
			Node::List(statements, bracket, separator)
		}
		other => other.map_children(|child| with_tables(child, tables, file, open)),
	}
}

fn table_statements(statement: Node, tables: &HashMap<String, Table>, file: &str, open: &mut Vec<String>) -> Vec<Node> {
	if let Some(lowered) = opened(&statement, tables, file, open).or_else(|| inserted(&statement, tables, file)).or_else(|| saved(&statement, tables, file)) {
		return lowered;
	}
	let updates = written_through(&statement, tables, file);
	[vec![with_tables(statement, tables, file, open)], updates].concat()
}

/// `people: [Person] = database.people` as the list of the table's rows, each an instance with its id; then the
/// one-to-many lists of the open tables its rows point back to
fn opened(statement: &Node, tables: &HashMap<String, Table>, file: &str, open: &mut Vec<String>) -> Option<Vec<Node>> {
	let Node::Key(target, Op::Assign, source) = statement.drop_meta() else { return None };
	let Node::Key(variable, Op::Colon, _) = target.drop_meta() else { return None };
	let variable = variable.drop_meta().name();
	let table = tables.get(&variable)?;
	database_table(source)?;
	if let Some((field, unopened)) = table.references.iter().find(|(_, referenced)| !open.contains(referenced)) {
		let message = format!("register {unopened} before {variable}: {}.{field} points to its rows", table.class);
		return Some(vec![crate::diagnostic::Diagnostic::at(statement, message).into_error()]);
	}
	if let Some(members) = table.members.iter().find(|members| members.back.is_none()) {
		let message = format!("{}.{} lists rows of {}, whose class has no field of class {} pointing back", table.class, members.field, members.table, table.class);
		return Some(vec![crate::diagnostic::Diagnostic::at(statement, message).into_error()]);
	}
	open.push(variable.clone());
	// the row is [id, column values…] in the order of the columns
	let arguments = constructor_arguments(table, ROW);
	let schema = Node::List(column_fields(table).into_iter().map(|(name, field_type, default)| {
		Node::List(vec![Node::Text(name), Node::Text(field_type), default.unwrap_or(Node::Empty)], Bracket::Square, Separator::Space)
	}).collect(), Bracket::Square, Separator::Space);
	let code = format!("[{class}({arguments}) for {ROW} in std_io(\"table\", \"open\", [{name:?}, {SCHEMA_PLACEHOLDER}, {file:?}])]",
		class = table.class, arguments = arguments.join(", "), name = table.name);
	let rows = generated(&code, [(SCHEMA_PLACEHOLDER, schema)]);
	let filled = tables.iter().filter(|(owner, _)| open.contains(owner)).flat_map(|(owner, owner_table)| members_filled(owner, owner_table, &variable));
	Some(std::iter::once(Node::Key(Box::new(target.drop_meta().clone()), Op::Assign, Box::new(rows))).chain(filled).collect())
}

/// `members: [Person]` of each team: the rows of people whose `team` is that team (`members` the table `owner`'s list
/// field of the rows of `table`)
fn members_filled(owner: &str, owner_table: &Table, table: &str) -> Vec<Node> {
	owner_table.members.iter().filter(|members| members.table == table).filter_map(|members| {
		let back = members.back.as_ref()?;
		Some(generated(&format!("for {OWNER} in {owner} {{ {OWNER}.{field} = [{MEMBER} for {MEMBER} in {table} if {MEMBER}.{back}.{ID_FIELD} == {OWNER}.{ID_FIELD}] }}",
			field = members.field), []))
	}).collect()
}

/// The code with each placeholder as its node and the generated variables named
fn generated<const N: usize>(code: &str, placeholders: [(&str, Node); N]) -> Node {
	let names = GENERATED_NAMES.iter().map(|(written, name)| (written.to_string(), Node::Symbol(name.to_string())));
	crate::law::substitute(&parse(code), &names.chain(placeholders.map(|(placeholder, node)| (placeholder.to_string(), node))).collect())
}

/// The table's columns besides id, in field order
fn columns_of(table: &Table) -> Vec<String> {
	column_fields(table).into_iter().map(|(name, _, _)| name).collect()
}

/// The fields kept in columns besides id (not one-to-many lists), a foreign key as the int id of its row
fn column_fields(table: &Table) -> Vec<(String, String, Option<Node>)> {
	table.fields.iter().filter(|(name, _, _)| name != ID_FIELD && !table.is_members(name)).map(|(name, field_type, default)| match table.reference(name) {
		Some(_) => (name.clone(), NUMERIC_TYPES[0].to_string(), None),
		None => (name.clone(), field_type.clone(), default.clone()),
	}).collect()
}

/// The row of table `target` whose id is the value of `id` (the instance a foreign key points to)
fn referenced(target: &str, id: &str) -> String {
	format!("[{REFERENCED} for {REFERENCED} in {target} if {REFERENCED}.{ID_FIELD} == {id}]#1")
}

fn implicit_id(table: &Table) -> bool {
	!table.fields.iter().any(|(name, _, _)| name == ID_FIELD)
}

/// `people.add(p)`: added to the list and inserted as a row, whose id p takes; a foreign key stores the id of its row
/// (an instance without one is an error) and p joins that row's one-to-many list of it
fn inserted(statement: &Node, tables: &HashMap<String, Table>, file: &str) -> Option<Vec<Node>> {
	let Node::Key(list, Op::Dot, call) = statement.drop_meta() else { return None };
	let list = list.drop_meta().name();
	let table = tables.get(&list)?;
	let Node::List(parts, _, _) = call.drop_meta() else { return None };
	let [word, value] = parts.as_slice() else { return None };
	if word.drop_meta().name() != ADD_WORD {
		return None;
	}
	let columns = columns_of(table);
	let names: Vec<String> = columns.iter().map(|column| format!("{column:?}")).collect();
	let values: Vec<String> = columns.iter().map(|column| column_value(table, ADDED, column)).collect();
	let checks = table.references.iter().map(|(field, target)| format!(
		"if {ADDED}.{field}.{ID_FIELD} == 0 {{ raise \"{class}.{field} is no row of {target}: add it to its table first\" }}\n", class = table.class));
	let joined = table.references.iter().flat_map(|(field, target)| tables[target].members.iter()
		.filter(|members| members.table == list && members.back.as_ref() == Some(field))
		.map(move |members| format!("\n{OWNER} = {ADDED}.{field}\n{OWNER}.{}.add({ADDED})", members.field)));
	let code = format!("{ADDED} = {VALUE_PLACEHOLDER}\n{checks}{list}.add({ADDED})\n{ADDED}.{ID_FIELD} = std_io(\"table\", \"insert\", [{table:?}, [{names}], [{values}], {file:?}]){joined}",
		checks = checks.collect::<String>(), table = table.name, names = names.join(" "), values = values.join(" "), joined = joined.collect::<String>());
	let lowered = generated(&code, [(VALUE_PLACEHOLDER, value.clone())]);
	Some(lowered.children())
}

/// What the column of `instance` stores: its field, the id of the row of a foreign key
fn column_value(table: &Table, instance: &str, column: &str) -> String {
	match table.reference(column) {
		Some(_) => format!("{instance}.{column}.{ID_FIELD}"),
		None => format!("{instance}.{column}"),
	}
}

/// After `p.age += 1` (any assignment of a field of a registered class): the change written to p's row, when p has one
fn written_through(statement: &Node, tables: &HashMap<String, Table>, file: &str) -> Vec<Node> {
	let Node::Key(target, op, _) = statement.drop_meta() else { return vec![] };
	if *op != Op::Assign && !op.is_compound_assign() {
		return vec![];
	}
	let Node::Key(instance, Op::Dot, field) = target.drop_meta() else { return vec![] };
	let (Node::Symbol(instance), field) = (instance.drop_meta(), field.drop_meta().name()) else { return vec![] };
	tables.values().filter(|table| field != ID_FIELD && columns_of(table).contains(&field))
		.map(|table| parse(&format!("if {instance} is {class} and {instance}.{ID_FIELD} > 0 {{ {update} }}",
			class = table.class, update = column_update(table, instance, &field, file)))).collect()
}

/// The UPDATE of one column of `instance`'s row
fn column_update(table: &Table, instance: &str, column: &str, file: &str) -> String {
	format!("std_io(\"table\", \"update\", [{name:?}, {instance}.{ID_FIELD}, {column:?}, {value}, {file:?}])",
		name = table.name, value = column_value(table, instance, column))
}

/// `save p`: every column of p's row written, the value p; an instance of a table's class without a row is an error
fn saved(statement: &Node, tables: &HashMap<String, Table>, file: &str) -> Option<Vec<Node>> {
	let Node::List(parts, _, Separator::Space) = statement.drop_meta() else { return None };
	let [word, value] = parts.as_slice() else { return None };
	if word.drop_meta().name() != SAVE_WORD || tables.is_empty() {
		return None;
	}
	let writes = tables.iter().map(|(variable, table)| {
		let updates: Vec<String> = columns_of(table).iter().map(|column| column_update(table, SAVED, column, file)).collect();
		format!("if {SAVED} is {class} {{\nif {SAVED}.{ID_FIELD} == 0 {{ raise \"save: this {class} has no row: add it to {name} first\" }}\n{updates}\n}}\n",
			class = table.class, name = variable, updates = updates.join("\n"))
	});
	let code = format!("{SAVED} = {VALUE_PLACEHOLDER}\n{writes}{SAVED}", writes = writes.collect::<String>());
	Some(generated(&code, [(VALUE_PLACEHOLDER, value.clone())]).children())
}
