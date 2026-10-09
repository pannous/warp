//! Plain classes as database tables (card orm, notes/orm.md step 1): `people: [Person] = database.people` registers the
//! table people for the class Person. Registering creates the table or migrates it to the class's fields
//! (`std_io("table", "migrate", …)`); the list loads the rows at its first read (`people·load()`, notes/orm.md
//! Loading), `people.add(p)` inserts p and gives it its row's id, and a field change
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
/// `people.remove(p)` deletes p's row, p keeping its fields without one (id 0)
const REMOVE_WORD: &str = "remove";
const GLOBAL_WORD: &str = "global";
/// `@was(old) name: text`: the field's column was called old, so the table's column is renamed
const RENAMED_MARK: &str = "was";
/// `save p` writes every column of p's row (field changes are written through already, so it changes nothing then)
const SAVE_WORD: &str = "save";
/// the generated code's variables, written as these placeholders (`·` would parse as a product)
const ROW: &str = "table_row";
/// the rows a table's open reads, and the rows a foreign key's id matches
const ROWS: &str = "table_rows";
const MATCHES: &str = "table_matches";
const ADDED: &str = "table_added";
const REMOVED: &str = "table_removed";
/// a foreign key's row, and a row of a one-to-many getter
const REFERENCED: &str = "table_referenced";
const MEMBER: &str = "table_member";
const SAVED: &str = "table_saved";
/// the instances given a row before the table loaded, of a loading row's id: the loaded list holds that instance
const KNOWN: &str = "table_known";
/// the position of `people#i`
const POSITION: &str = "table_position";
/// the instance a row read before loading makes
const MADE: &str = "table_made";
const GENERATED_NAMES: [(&str, &str); 12] = [(POSITION, "table·position"), (MADE, "table·made"), (ROW, "table·row"), (ROWS, "table·rows"), (MATCHES, "table·matches"), (ADDED, "table·added"), (REMOVED, "table·removed"), (ARGUMENTS, "table·arguments"),
	(REFERENCED, "table·referenced"), (MEMBER, "table·member"), (SAVED, "table·saved"), (KNOWN, "table·known")];
/// Each table's lazy parts (notes/orm.md Loading), `people·load` of people: the function giving the list, loading its
/// rows on the first call; the count, SELECT COUNT(*) until then; the add, inserting without loading; the reset of a
/// route reading the table anew; the element `people#i`, reading its one row until then; the element of a loop over the
/// table, reading a page of rows until then; the instance of a row read before loading; whether the rows are loaded;
/// the instances added or read before; the page read last and its start position
const LOAD: &str = "table_load";
const COUNTED: &str = "table_count";
const ADDING: &str = "table_add";
const REMOVING: &str = "table_remove";
const RESET: &str = "table_reset";
const LOADED: &str = "table_loaded";
const MET: &str = "table_met";
const ELEMENT: &str = "table_at";
const STREAMED: &str = "table_streamed";
const KEPT: &str = "table_kept";
const PAGE: &str = "table_page";
const START: &str = "table_start";
const LAZY_PARTS: [(&str, &str); 12] = [(LOAD, "load"), (COUNTED, "count"), (ADDING, "add"), (REMOVING, "remove"), (RESET, "reset"), (ELEMENT, "at"), (STREAMED, "streamed"),
	(KEPT, "kept"), (LOADED, "loaded"), (MET, "met"), (PAGE, "page"), (START, "start")];
/// The rows a loop over an unloaded table reads at once (notes/orm.md Loading)
const PAGE_SIZE: usize = 100;
/// `count(people)`, `people.count`: counted without loading
const COUNT_WORDS: [&str; 4] = ["count", "size", "length", "len"];
const FOR_WORD: &str = "for";
/// a paged loop's count and position, `p·end` and `p·position` of `for p in people`
const LOOP_END: &str = "table_loop_end";
const LOOP_POSITION: &str = "table_loop_position";
const IN_WORD: &str = "in";
const SCHEMA_PLACEHOLDER: &str = "table_schema";
const READ_PLACEHOLDER: &str = "table_read";
const ROWS_PLACEHOLDER: &str = "table_rows_read";
const COUNT_PLACEHOLDER: &str = "table_count_read";
const ROW_PLACEHOLDER: &str = "table_row_read";
const PAGE_PLACEHOLDER: &str = "table_page_read";
const VALUE_PLACEHOLDER: &str = "table_value";
/// `red.players.add(p)`: the instance whose one-to-many field is added to
const OWNER_PLACEHOLDER: &str = "table_owner";
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
/// `Team?`: an optional field
const OPTIONAL_MARK: char = '?';
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
	/// fields marked `@was(old)`: the field and its column's old name
	renamed: Vec<(String, String)>,
}

/// `members: [Person]` of Team: the rows of `table` (people) whose field `back` (team) is the team
struct Members {
	field: String,
	table: String,
	back: Option<String>,
	/// whether `back` is optional (`team: Team?`), so a row may point to no team
	optional_back: bool,
}

impl Table {
	fn reference(&self, field: &str) -> Option<&str> {
		self.references.iter().find(|(name, _)| name == field).map(|(_, table)| table.as_str())
	}

	fn is_members(&self, field: &str) -> bool {
		self.members.iter().any(|members| members.field == field)
	}

	/// `team: Team?`: a foreign key that may point to no row, ø
	fn is_optional(&self, field: &str) -> bool {
		self.fields.iter().any(|(name, field_type, _)| name == field && field_type.ends_with(OPTIONAL_MARK))
	}
}

/// The class of a field's type: `Team` of `Team?`
fn class_of(field_type: &str) -> &str {
	field_type.trim_end_matches(OPTIONAL_MARK)
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
	with_lazy_reads(with_tables(with_row_fields(program, &tables), &tables, &tables_file(), &mut vec![]), &tables, None)
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
			if let Some(target) = variable_of.get(class_of(field_type)) {
				table.references.push((field.clone(), target.clone()));
			}
			let element = field_type.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or_default();
			if let Some(target) = variable_of.get(element) {
				let back_field = fields_of[element].iter().find(|(_, back_type)| class_of(back_type) == table.class);
				let (back, optional_back) = (back_field.map(|(name, _)| name.clone()), back_field.is_some_and(|(_, back_type)| back_type.ends_with(OPTIONAL_MARK)));
				table.members.push(Members { field: field.clone(), table: target.clone(), back, optional_back });
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
					Some(_) => generated(&referenced(table, &name, &format!("{ARGUMENTS}#{index}")), []),
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
	let column = |name: &str| column_cell(table, row, name);
	table.fields.iter().filter(|(name, _, _)| !table.is_members(name)).map(|(name, _, _)| match (name == ID_FIELD, table.reference(name)) {
		(true, _) => format!("{row}#1"),
		(false, Some(_)) => referenced(table, name, &column(name)),
		_ => column(name),
	}).chain(implicit_id(table).then(|| format!("{row}#1"))).collect()
}

/// The id a foreign key of `instance` stores: its row's, 0 for an optional key holding ø
fn key_id(instance: &str, field: &str, optional: bool) -> String {
	match optional {
		true => format!("(if {instance}.{field} == ø then 0 else {instance}.{field}.{ID_FIELD})"),
		false => format!("{instance}.{field}.{ID_FIELD}"),
	}
}

/// The cell of column `name` in `row`, a row being `[id, column values…]`
fn column_cell(table: &Table, row: &str, name: &str) -> String {
	format!("{row}#{}", 2 + columns_of(table).iter().position(|column| column == name).unwrap_or_default())
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
	let body = classes.get(&class)?;
	let fields = crate::class_methods::field_declarations(body);
	let renamed = crate::class_methods::fields_marked(body, RENAMED_MARK).into_iter().map(|(field, old)| (field, old.drop_meta().name())).collect();
	Some((variable.drop_meta().name(), Table { name, class, fields, references: vec![], members: vec![], renamed }))
}

/// The table `database.t` names
fn database_table(source: &Node) -> Option<String> {
	let Node::Key(store, Op::Dot, table) = source.drop_meta() else { return None };
	DATABASE_WORDS.contains(&store.drop_meta().name().as_str()).then(|| table.drop_meta().name())
}

/// Each registered class with the field `id: int = 0` unless it has one (0 until the instance has a row), and its
/// one-to-many fields as getters
fn with_row_fields(node: Node, tables: &HashMap<String, Table>) -> Node {
	match node {
		Node::Type { name, body } => match tables.values().find(|table| table.class == name.drop_meta().name()) {
			Some(table) => Node::Type { name, body: Box::new(with_id(with_member_getters(*body, table))) },
			None => Node::Type { name, body },
		},
		other => other.map_children(|child| with_row_fields(child, tables)),
	}
}

/// `players: [Person]` of Team as the getter of the people pointing back, a query at each read (lazy): a moved row is
/// seen at once
fn with_member_getters(body: Node, table: &Table) -> Node {
	let getter = |item: Node| {
		let Node::Key(field, Op::Colon, _) = item.drop_meta() else { return item };
		let Some(members) = table.members.iter().find(|members| members.field == field.drop_meta().name()) else { return item };
		let Some(back) = &members.back else { return item };
		generated(&format!("{field} := {{ [{MEMBER} for {MEMBER} in {LOAD}() if {key} == {ID_FIELD}] }}", key = key_id(MEMBER, back, members.optional_back),
			field = members.field), lazy_names(&members.table))
	};
	match body {
		Node::List(items, Bracket::Curly, separator) if !table.members.is_empty() => Node::List(items.into_iter().map(getter).collect(), Bracket::Curly, separator),
		body => body,
	}
}

fn with_id(body: Node) -> Node {
	if has_id(&body) {
		return body;
	}
	let id = parse(&format!("{ID_FIELD}: int = 0"));
	match body {
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
	}
}

fn has_id(body: &Node) -> bool {
	crate::class_methods::field_declarations(body).iter().any(|(name, _, _)| name == ID_FIELD)
}

/// The registrations, inserts and field changes of the tables as their table calls, in every block; `open` the tables
/// registered so far, as a foreign key reads the rows of its table
fn with_tables(node: Node, tables: &HashMap<String, Table>, file: &str, open: &mut Vec<String>) -> Node {
	if let Some(lowered) = saved(&node, tables, file).or_else(|| added_to_members(&node, tables, file)) {
		return lowered;
	}
	match node {
		Node::List(statements, bracket, separator @ (Separator::Semicolon | Separator::Newline)) => {
			let statements = statements.into_iter().flat_map(|statement| table_statements(statement, tables, file, open)).collect();
			Node::List(statements, bracket, separator)
		}
		// a block of one statement: `if count(teams) == 0 { teams.add(red) }`
		Node::List(statements, Bracket::Curly, separator) if statements.len() == 1 => {
			let statements: Vec<Node> = statements.into_iter().flat_map(|statement| table_statements(statement, tables, file, open)).collect();
			let separator = if statements.len() == 1 { separator } else { Separator::Semicolon };
			Node::List(statements, Bracket::Curly, separator)
		}
		other => other.map_children(|child| with_tables(child, tables, file, open)),
	}
}

fn table_statements(statement: Node, tables: &HashMap<String, Table>, file: &str, open: &mut Vec<String>) -> Vec<Node> {
	if let Some(lowered) = opened(&statement, tables, file, open).or_else(|| inserted(&statement, tables, file)) {
		return lowered;
	}
	let updates = written_through(&statement, tables, file);
	[vec![with_tables(statement, tables, file, open)], updates].concat()
}

/// `people: [Person] = database.people` as the list of the table's rows, each an instance with its id; then the
/// one-to-many lists of the open tables its rows point back to. A plain `people = database.people` of a registered
/// people opens it again (a served route reading it at each request)
fn opened(statement: &Node, tables: &HashMap<String, Table>, file: &str, open: &mut Vec<String>) -> Option<Vec<Node>> {
	let Node::Key(target, Op::Assign, source) = statement.drop_meta() else { return None };
	let variable = match target.drop_meta() {
		Node::Key(variable, Op::Colon, _) => variable.drop_meta(),
		variable @ Node::Symbol(_) => variable,
		_ => return None,
	};
	let variable = variable.name();
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
	// a column is [name type default] and its old name when renamed
	let schema = Node::List(column_fields(table).into_iter().map(|(name, field_type, default)| {
		let old_name = table.renamed.iter().find(|(field, _)| *field == name).map(|(_, old)| Node::Text(old.clone()));
		let column = [Node::Text(name), Node::Text(field_type), default.unwrap_or(Node::Empty)].into_iter().chain(old_name);
		Node::List(column.collect(), Bracket::Square, Separator::Space)
	}).collect(), Bracket::Square, Separator::Space);
	if !matches!(target.drop_meta(), Node::Key(_, Op::Colon, _)) {
		// `people = database.people` again (a served route at each request): the next read loads the rows anew; an
		// assignment still, so a block of it and a value stays a block (`{ p = …; p#1 }`), no list
		let reset = called(lazy_name(&variable, "reset"), None);
		return Some(vec![Node::Key(Box::new(target.drop_meta().clone()), Op::Assign, Box::new(reset))]);
	}
	let table_call_with = |member: &str, extra: &str| generated(&format!("std_io(\"table\", {member:?}, [{:?}, {SCHEMA_PLACEHOLDER}, {file:?}{extra}])", table.name), [(SCHEMA_PLACEHOLDER, schema.clone())]);
	let table_call = |member: &str| table_call_with(member, "");
	let class = &table.class;
	let required: Vec<&str> = table.references.iter().map(|(field, _)| field.as_str()).filter(|field| !table.is_optional(field)).collect();
	// a required key without its row (deleted, or the 0 of a column added for it) leaves its row out, reported
	let found = |field: &str| format!("count({}) > 0", matching_rows(table, field, &column_cell(table, ROW, field)));
	let warnings: String = required.iter().map(|field| format!(
		"if not ({found}) {{ warning(\"{variable} row \" + {ROW}#1 + \": {field} \" + {cell} + \" is no row of {target}; the row is left out (declare {field}: {class}? to keep it, with ø)\") }}\n",
		found = found(field), cell = column_cell(table, ROW, field), target = table.reference(field).unwrap_or_default(),
		class = class_of(&table.fields.iter().find(|(name, _, _)| name == field).map(|(_, field_type, _)| field_type.clone()).unwrap_or_default()))).collect();
	let checks = match warnings.is_empty() {
		true => String::new(),
		false => format!("for {ROW} in {ROWS} {{\n{warnings}}}\n"),
	};
	let kept = match required.is_empty() {
		true => String::new(),
		false => format!(" if {}", required.iter().map(|field| found(field)).collect::<Vec<_>>().join(" and ")),
	};
	let known = format!("{KNOWN} = [{REFERENCED} for {REFERENCED} in {MET} if {REFERENCED}.{ID_FIELD} == {ROW}#1]");
	let constructed = format!("{class}({})", arguments.join(", "));
	let instance = format!("({known}; if {KNOWN} then {KNOWN}#1 else {constructed})");
	// the rows a required key leaves out are counted and positioned only by loading
	let (unloaded_count, unloaded_element, unloaded_streamed) = match required.is_empty() {
		true => (COUNT_PLACEHOLDER.to_string(), format!("{{
{ROWS} = {ROW_PLACEHOLDER}
if not {ROWS} then {LOAD}()#{POSITION} else {KEPT}({ROWS}#1)
}}"), format!("{{
if {POSITION} < {START} or {POSITION} >= {START} + count({PAGE}) {{
{PAGE} = [{KEPT}({ROW}) for {ROW} in {PAGE_PLACEHOLDER}]
{START} = {POSITION}
}}
{PAGE}#({POSITION} - {START} + 1)
}}")),
		false => (format!("count({LOAD}())"), format!("{LOAD}()#{POSITION}"), format!("{LOAD}()#{POSITION}")),
	};
	let code = format!("{LOADED} = no
{MET}: [{class}] = []
{PAGE}: [{class}] = []
{START} = 0
{READ_PLACEHOLDER}
{LOAD}() := {{
global {variable}
global {LOADED}
global {MET}
if not {LOADED} {{
{ROWS} = {ROWS_PLACEHOLDER}
{checks}{variable} = [{instance} for {ROW} in {ROWS}{kept}]
{LOADED} = yes
}}
{variable}
}}
{COUNTED}() := {{
global {variable}
global {LOADED}
if {LOADED} then count({variable}) else {unloaded_count}
}}
{ADDING}({ADDED}) := {{
global {variable}
global {LOADED}
global {MET}
if {LOADED} {{ {variable}.add({ADDED}) }} else {{ {MET}.add({ADDED}) }}
{ADDED}
}}
{REMOVING}({REMOVED}) := {{
global {variable}
global {LOADED}
global {MET}
global {PAGE}
if {LOADED} {{ if {variable} {{ {variable}.remove({REMOVED}) }} }}
{MET}.remove({REMOVED})
{PAGE} = []
{REMOVED}
}}
{KEPT}({ROW}) := {{
global {MET}
{known}
if {KNOWN} then {KNOWN}#1 else {{
{MADE} = {constructed}
{MET}.add({MADE})
{MADE}
}}
}}
{ELEMENT}({POSITION}) := {{
global {variable}
global {LOADED}
if {LOADED} or {POSITION} < 1 then {LOAD}()#{POSITION} else {unloaded_element}
}}
{STREAMED}({POSITION}) := {{
global {variable}
global {LOADED}
global {PAGE}
global {START}
if {LOADED} then {variable}#{POSITION} else {unloaded_streamed}
}}
{RESET}() := {{
global {LOADED}
global {MET}
global {PAGE}
{LOADED} = no
{MET} = []
{PAGE} = []
[]
}}");
	let placeholders = [(READ_PLACEHOLDER, table_call("migrate")), (ROWS_PLACEHOLDER, table_call("rows")), (COUNT_PLACEHOLDER, table_call("count")),
		(ROW_PLACEHOLDER, table_call_with("page", &format!(", {POSITION}, 1"))), (PAGE_PLACEHOLDER, table_call_with("page", &format!(", {POSITION}, {PAGE_SIZE}")))].into_iter().chain(lazy_names(&variable));
	let empty = parse("[]");
	Some([vec![Node::Key(Box::new(target.drop_meta().clone()), Op::Assign, Box::new(empty))], generated(&code, placeholders).children()].concat())
}


/// The code with each placeholder as its node and the generated variables named
fn generated<'a>(code: &str, placeholders: impl IntoIterator<Item = (&'a str, Node)>) -> Node {
	let names = GENERATED_NAMES.iter().map(|(written, name)| (written.to_string(), Node::Symbol(name.to_string())));
	crate::law::substitute(&parse(code), &names.chain(placeholders.into_iter().map(|(placeholder, node)| (placeholder.to_string(), node))).collect())
}

/// The placeholders of a table's lazy parts as their names: `table_load` of people is `people·load`
fn lazy_names(variable: &str) -> Vec<(&'static str, Node)> {
	LAZY_PARTS.iter().map(|(placeholder, part)| (*placeholder, Node::Symbol(lazy_name(variable, part)))).collect()
}

fn lazy_name(variable: &str, part: &str) -> String {
	format!("{variable}·{part}")
}

/// The call `name()` or `name(argument)`
fn called(name: String, argument: Option<Node>) -> Node {
	let code = match argument { Some(_) => format!("{FUNCTION_PLACEHOLDER}({VALUE_PLACEHOLDER})"), None => format!("{FUNCTION_PLACEHOLDER}()") };
	generated(&code, [(FUNCTION_PLACEHOLDER, Node::Symbol(name))].into_iter().chain(argument.map(|value| (VALUE_PLACEHOLDER, value))))
}

/// Reads of a table's list as its load (`people` → `people·load()`), `count(people)` as its count, `people.add(p)` as
/// its add and `people#i` as its element, which load no rows; a table's own lazy functions keep its list (`own`). Registrations, `global people`,
/// assigned lists and field names (`x.people`) stay
fn with_lazy_reads(node: Node, tables: &HashMap<String, Table>, own: Option<&str>) -> Node {
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
		Node::List(parts, _, _) if parts.len() == 2 && COUNT_WORDS.contains(&parts[0].drop_meta().name().as_str()) && table_of(&parts[1]).is_some() => {
			called(lazy_name(&table_of(&parts[1]).unwrap_or_default(), "count"), None)
		}
		Node::List(parts, _, _) if parts.len() == 5 && parts[0].drop_meta().name() == FOR_WORD && parts[2].drop_meta().name() == IN_WORD
			&& matches!(parts[1].drop_meta(), Node::Symbol(_)) && table_of(&parts[3]).is_some() && matches!(parts[4].drop_meta(), Node::List(_, Bracket::Curly, _)) => {
			let variable = table_of(&parts[3]).unwrap_or_default();
			paged_loop(&parts[1].drop_meta().name(), &variable, &tables[&variable].class, rewrite(parts[4].clone()))
		}
		Node::Key(left, Op::Hash, position) if table_of(&left).is_some() => called(lazy_name(&table_of(&left).unwrap_or_default(), "at"), Some(rewrite(*position))),
		Node::Key(left, Op::Dot, right) => match (table_of(&left), right.drop_meta()) {
			(Some(variable), Node::Symbol(word)) if COUNT_WORDS.contains(&word.as_str()) => called(lazy_name(&variable, "count"), None),
			(Some(variable), Node::List(parts, _, _)) if parts.len() == 2 && [ADD_WORD, REMOVE_WORD].contains(&parts[0].drop_meta().name().as_str()) => {
				called(lazy_name(&variable, &parts[0].drop_meta().name()), Some(rewrite(parts[1].clone())))
			}
			(_, Node::Symbol(_)) => Node::Key(Box::new(rewrite(*left)), Op::Dot, right),
			_ => Node::Key(Box::new(rewrite(*left)), Op::Dot, Box::new(rewrite(*right))),
		},
		Node::Key(target, op, value) if (op == Op::Assign || op.is_compound_assign()) && table_of(declared(&target)).is_some() => Node::Key(target, op, Box::new(rewrite(*value))),
		_ => node.map_children(rewrite),
	}
}

/// `for p in people body` over an unloaded table walks positions up to its count, each element read from a page
/// (`people·streamed`); the count is taken once, so rows the body adds are not walked
fn paged_loop(element: &str, variable: &str, class: &str, body: Node) -> Node {
	let names = || [(LOOP_END, "end"), (LOOP_POSITION, "position")].map(|(placeholder, part)| (placeholder, Node::Symbol(format!("{element}·{part}"))))
		.into_iter().chain(lazy_names(variable));
	// the body's statements follow the element's in one block: a nested block of expressions would read as a list
	let element_read = generated(&format!("{element}: {class} = {STREAMED}({LOOP_POSITION})"), names());
	let statements = Node::List([vec![element_read], body.children()].concat(), Bracket::Curly, Separator::Semicolon);
	let code = format!("{LOOP_END} = {COUNTED}()\nfor {LOOP_POSITION} in 1 to {LOOP_END} {VALUE_PLACEHOLDER}");
	let lowered = generated(&code, names().chain([(VALUE_PLACEHOLDER, statements)]));
	Node::List(lowered.children(), Bracket::None, Separator::Semicolon)
}

/// The variable an assignment's target names: `people` of `people: [Person]`
fn declared(target: &Node) -> &Node {
	match target.drop_meta() {
		Node::Key(variable, Op::Colon, _) => variable,
		_ => target,
	}
}

/// `people·load() := …` and the other lazy functions of a table: its variable
fn lazy_definition(node: &Node, tables: &HashMap<String, Table>) -> Option<String> {
	let Node::Key(head, Op::Define, _) = node.drop_meta() else { return None };
	let name = match head.drop_meta() {
		Node::List(parts, _, _) => parts.first()?.drop_meta().name(),
		other => other.name(),
	};
	let (variable, part) = name.rsplit_once('·')?;
	(tables.contains_key(variable) && LAZY_PARTS.iter().any(|(_, lazy_part)| *lazy_part == part)).then(|| variable.to_string())
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

/// The row whose id is the value of `id` (the instance the foreign key `field` points to); ø for an optional key
/// without its row
fn referenced(table: &Table, field: &str, id: &str) -> String {
	let matches = matching_rows(table, field, id);
	match table.is_optional(field) {
		true => format!("({MATCHES} = {matches}; if {MATCHES} then {MATCHES}#1 else ø)"),
		false => format!("{matches}#1"),
	}
}

/// The rows of the table the foreign key `field` points to whose id is the value of `id`: one, or none when dangling
fn matching_rows(table: &Table, field: &str, id: &str) -> String {
	let target = table.reference(field).unwrap_or_default();
	format!("[{REFERENCED} for {REFERENCED} in {target} if {REFERENCED}.{ID_FIELD} == {id}]")
}

fn implicit_id(table: &Table) -> bool {
	!table.fields.iter().any(|(name, _, _)| name == ID_FIELD)
}

/// `people.add(p)`: added to the list and inserted as a row, whose id p takes; a foreign key stores the id of its row
/// (an instance without one is an error)
fn inserted(statement: &Node, tables: &HashMap<String, Table>, file: &str) -> Option<Vec<Node>> {
	let (list, table, word, value) = table_mutation(statement, tables)?;
	let code = match word.as_str() {
		ADD_WORD => format!("{ADDED} = {VALUE_PLACEHOLDER}\n{insert}", insert = insert_code(table, &list, file)),
		REMOVE_WORD => format!("{REMOVED} = {VALUE_PLACEHOLDER}\n{list}.remove({REMOVED})
if {REMOVED}.{ID_FIELD} > 0 {{ std_io(\"table\", \"delete\", [{name:?}, {REMOVED}.{ID_FIELD}, {file:?}]); {REMOVED}.{ID_FIELD} = 0 }}", name = table.name),
		_ => return None,
	};
	let lowered = generated(&code, [(VALUE_PLACEHOLDER, value.clone())]);
	Some(lowered.children())
}

/// `people.add(p)`, `people.remove(p)` of a table: its list, the table, the word and p
fn table_mutation<'a>(statement: &'a Node, tables: &'a HashMap<String, Table>) -> Option<(String, &'a Table, String, &'a Node)> {
	let Node::Key(list, Op::Dot, call) = statement.drop_meta() else { return None };
	let list = list.drop_meta().name();
	let table = tables.get(&list)?;
	let Node::List(parts, _, _) = call.drop_meta() else { return None };
	let [word, value] = parts.as_slice() else { return None };
	Some((list, table, word.drop_meta().name(), value))
}

/// `people.add(ADDED)` and its INSERT, ADDED taking the row's id; a foreign key without its row is an error
fn insert_code(table: &Table, list: &str, file: &str) -> String {
	let columns = columns_of(table);
	let names: Vec<String> = columns.iter().map(|column| format!("{column:?}")).collect();
	let values: Vec<String> = columns.iter().map(|column| column_value(table, ADDED, column)).collect();
	let checks = table.references.iter().map(|(field, target)| format!(
		"if {ADDED}.{field} != ø and {ADDED}.{field}.{ID_FIELD} == 0 {{ raise \"{class}.{field} is no row of {target}: add it to its table first\" }}\n", class = table.class));
	format!("{checks}{list}.add({ADDED})\n{ADDED}.{ID_FIELD} = std_io(\"table\", \"insert\", [{table:?}, [{names}], [{values}], {file:?}])",
		checks = checks.collect::<String>(), table = table.name, names = names.join(" "), values = values.join(" "))
}

/// `red.players.add(p)` of a one-to-many field, anywhere (`d.team.players.add(d)` too): p's key points to red, written
/// to p's row, or p inserted into people when it has none (card orm-nested); the value p
fn added_to_members(node: &Node, tables: &HashMap<String, Table>, file: &str) -> Option<Node> {
	let Node::Key(members_of, Op::Dot, call) = node.drop_meta() else { return None };
	let Node::Key(owner, Op::Dot, field) = members_of.drop_meta() else { return None };
	let Node::List(parts, _, _) = call.drop_meta() else { return None };
	let [word, value] = parts.as_slice() else { return None };
	if word.drop_meta().name() != ADD_WORD {
		return None;
	}
	let field = field.drop_meta().name();
	let members = tables.values().flat_map(|table| &table.members).find(|members| members.field == field)?;
	let (back, table) = (members.back.as_ref()?, &tables[&members.table]);
	let code = format!("{ADDED} = {VALUE_PLACEHOLDER}\n{ADDED}.{back} = {OWNER_PLACEHOLDER}\nif {ADDED}.{ID_FIELD} > 0 {{ {update} }} else {{\n{insert}\n}}\n{ADDED}",
		update = column_update(table, ADDED, back, file), insert = insert_code(table, &members.table, file));
	let lowered = generated(&code, [(VALUE_PLACEHOLDER, value.clone()), (OWNER_PLACEHOLDER, *owner.clone())]);
	Some(Node::List(lowered.children(), Bracket::Round, Separator::Semicolon))
}

/// What the column of `instance` stores: its field, the id of the row of a foreign key
fn column_value(table: &Table, instance: &str, column: &str) -> String {
	match table.reference(column) {
		Some(_) => key_id(instance, column, table.is_optional(column)),
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

/// `save p`, anywhere (`print(save p)` of a standalone build too): every column of p's row written, the value p; an
/// instance of a table's class without a row is an error
fn saved(node: &Node, tables: &HashMap<String, Table>, file: &str) -> Option<Node> {
	let Node::List(parts, _, Separator::Space) = node.drop_meta() else { return None };
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
	Some(Node::List(generated(&code, [(VALUE_PLACEHOLDER, value.clone())]).children(), Bracket::Round, Separator::Semicolon))
}
