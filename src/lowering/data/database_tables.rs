//! Plain classes as database tables (card orm, notes/orm.md step 1): `stored people: [Person]` registers the
//! table people for the class Person. Registering creates the table or migrates it to the class's fields
//! (`std_io("table", "migrate", …)`); the list loads the rows at its first read (`people·load()`, notes/orm.md
//! Loading), `people.add(p)` inserts p and gives it its row's id, and a field change
//! `p.age += 1` of an instance with a row is written through (`std_io("table", "update", …)`). Natively the tables live
//! in `<program>.database.sqlite` (in memory for code without a file, database.rs); the browser keeps them in IndexedDB
//! (web/playground/host-files.js), where a filter stays the list comprehension over the rows.

use super::nodes::key;
use crate::node::{symbol, Bracket, Node, Separator};
use crate::numbers::units::static_units::si_quantity;
use crate::operators::Op;
use crate::stored_values::DATABASE_WORD;
use crate::warp_parser::parse;
use crate::words::ADD_WORD;
use crate::words::{FOR_WORD, GLOBAL_WORD, IN_WORD};
use std::collections::{BTreeMap, HashMap, HashSet};

mod queries;
pub use queries::*;
mod lazy_reads;
pub(crate) use lazy_reads::*;
mod row_writes;
use row_writes::*;

/// `app.warp` keeps its tables in `app.database.sqlite`; inline code's tables are this name alone (in memory)
pub const TABLES_FILE: &str = "database.sqlite";
/// The row id every registered class gets, unless it declares one
const ID_FIELD: &str = "id";
/// `first 3 of people`, `people.last(2)`: a slice at either end of a table
const END_WORDS: [&str; 2] = ["first", "last"];
const REMOVE_WORD: &str = "remove";
/// `@was(old) name: text`: the field's column was called old, so the table's column is renamed
const RENAMED_MARK: &str = "was";
/// `save p` writes every column of p's row (field changes are written through already, so it changes nothing then)
const SAVE_WORD: &str = "save";
/// `transaction { … }`: BEGIN, the block, COMMIT; ROLLBACK when the block fails, which then fails on (notes/orm.md)
const TRANSACTION_WORD: &str = "transaction";
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
/// a transaction's value, and the failure of its block
const TRANSACTION_VALUE: &str = "table_transaction";
const FAILURE: &str = "table_failure";
/// a restore's rows by their id, and a key of the identity map
const KNOWN: &str = "table_known";
const KEY: &str = "table_key";
/// the position of `people#i`
const POSITION: &str = "table_position";
/// the instance a row read before loading makes
const MADE: &str = "table_made";
/// a filter's SQL condition and its parameters
const CONDITION: &str = "table_condition";
const CONDITION_VALUES: &str = "table_condition_values";
/// a slice's 0-based start and exclusive end (`people#(11..20)` is 10 to 20), the start at 0 at least, and its rows
const FROM: &str = "table_from";
const UNTIL: &str = "table_until";
const FIRST: &str = "table_first";
const SLICE_ROWS: &str = "table_slice_rows";
/// the ids a page of rows' foreign keys point to, those not met yet
const KEYS: &str = "table_keys";
const MISSING: &str = "table_missing";
const GENERATED_NAMES: [(&str, &str); 23] = [
	(POSITION, "table·position"),
	(MADE, "table·made"),
	(ROW, "table·row"),
	(ROWS, "table·rows"),
	(MATCHES, "table·matches"),
	(ADDED, "table·added"),
	(REMOVED, "table·removed"),
	(ARGUMENTS, "table·arguments"),
	(REFERENCED, "table·referenced"),
	(MEMBER, "table·member"),
	(SAVED, "table·saved"),
	(KNOWN, "table·known"),
	(KEY, "table·key"),
	(TRANSACTION_VALUE, "table·transaction"),
	(FAILURE, "table·failure"),
	(CONDITION, "table·condition"),
	(CONDITION_VALUES, "table·condition_values"),
	(FROM, "table·from"),
	(UNTIL, "table·until"),
	(FIRST, "table·first"),
	(SLICE_ROWS, "table·slice_rows"),
	(KEYS, "table·keys"),
	(MISSING, "table·missing"),
];
/// Each table's lazy parts (notes/orm.md Loading), `people·load` of people: the function giving the list, loading its
/// rows on the first call; the count, SELECT COUNT(*) until then; the add, inserting without loading; the remove, deleting
/// the row and dropping its instance from the loaded list and the known instances; the reset of a
/// route reading the table anew; the element `people#i`, reading its one row until then; the element of a loop over the
/// table, reading a page of rows until then; the instance of a row read before loading; whether the rows are loaded;
/// the instances added or read before; the page read last and its start position; the restore after a rolled-back
/// transaction, giving each instance held its row's values again (id 0 when its row is gone) and loading anew; the
/// rows a filter keeps, read alone as their instances until the table loads (the loaded ones after); the instance of an id,
/// ø when no row has it, found in the instances met, which hold each loaded one; a slice, reading only its rows until
/// the table loads (card orm-paging); the rows of ids not met yet, read together for a page of rows pointing to them (card
/// orm-batched)
const LOAD: &str = "table_load";
const COUNTED: &str = "table_count";
const ADDING: &str = "table_add";
const REMOVING: &str = "table_remove";
const RESET: &str = "table_reset";
const RESTORE: &str = "table_restore";
const LOADED: &str = "table_loaded";
const MET: &str = "table_met";
const ELEMENT: &str = "table_at";
const STREAMED: &str = "table_streamed";
const KEPT: &str = "table_kept";
const PAGE: &str = "table_page";
const START: &str = "table_start";
const FOUND: &str = "table_found";
const OF: &str = "table_of";
const SLICED: &str = "table_sliced";
const FETCHED: &str = "table_fetched";
/// `people.table·found(condition, parameters)`: a filter's query until with_lazy_reads
const FOUND_WORD: &str = "table·found";
/// `teams.table_instance_of(id)`: the instance of an id until with_lazy_reads makes it `teams·of(id)`; likewise
/// `teams.table_fetched_ids(ids)`, `teams·fetched(ids)`
const INSTANCE_OF_WORD: &str = "table_instance_of";
const FETCHED_WORD: &str = "table_fetched_ids";
/// Rows whose keys point to more rows not met than this load the other table whole rather than by `id IN (…)`
const MOST_FETCHED: usize = 500;
const LAZY_PARTS: [(&str, &str); 17] = [
	(FOUND, "found"),
	(SLICED, "sliced"),
	(FETCHED, "fetched"),
	(OF, "of"),
	(LOAD, "load"),
	(COUNTED, "count"),
	(ADDING, "add"),
	(REMOVING, "remove"),
	(RESET, "reset"),
	(RESTORE, "restore"),
	(ELEMENT, "at"),
	(STREAMED, "streamed"),
	(KEPT, "kept"),
	(LOADED, "loaded"),
	(MET, "met"),
	(PAGE, "page"),
	(START, "start"),
];
/// The rows a loop over an unloaded table reads at once (notes/orm.md Loading)
const PAGE_SIZE: usize = 100;
/// a paged loop's count and position, `p·end` and `p·position` of `for p in people`
const LOOP_END: &str = "table_loop_end";
const LOOP_POSITION: &str = "table_loop_position";
const SCHEMA_PLACEHOLDER: &str = "table_schema";
const READ_PLACEHOLDER: &str = "table_read";
const ROWS_PLACEHOLDER: &str = "table_rows_read";
const COUNT_PLACEHOLDER: &str = "table_count_read";
const ROW_PLACEHOLDER: &str = "table_row_read";
const PAGE_PLACEHOLDER: &str = "table_page_read";
const SELECT_PLACEHOLDER: &str = "table_select_read";
const SLICE_PLACEHOLDER: &str = "table_slice_read";
const FROM_PLACEHOLDER: &str = "table_from_value";
/// `"id" IN (`, the start of the condition fetching rows by their ids
const IN_PLACEHOLDER: &str = "table_in";
const VALUE_PLACEHOLDER: &str = "table_value";
/// `red.players.add(p)`: the instance whose one-to-many field is added to
const OWNER_PLACEHOLDER: &str = "table_owner";
/// the instance whose field change written_through writes to its row
const INSTANCE_PLACEHOLDER: &str = "table_instance";
/// A filter's query (notes/orm.md step 2): the ids it keeps, its SQL condition and parameters, the functions it calls
const CONDITION_PLACEHOLDER: &str = "table_condition_text";
const PARAMETERS_PLACEHOLDER: &str = "table_parameters";
const FUNCTION_PLACEHOLDER: &str = "table_function";
const BODY_PLACEHOLDER: &str = "table_body";
/// The one parameter of a query's function: [id, columns…, the filter's values…] of a row
const ARGUMENTS: &str = "table_arguments";
const FUNCTION: &str = "table·call";
/// The SQL function a query calls a warp function through (database.rs)
pub(crate) const WARP_CALL: &str = "warp_call";
/// `Team?`: an optional field
pub(crate) const OPTIONAL_MARK: char = '?';
const NUMERIC_TYPES: [&str; 4] = ["int", "float", "number", "real"];
/// A bool field's column holds 0/1 (database.rs column types), read back as the bool
const BOOL_TYPE: &str = "bool";
/// What a filter's function should not do: not being sure to end (Div) or allocating is fine
const SIDE_EFFECTS: [crate::analyzer::effects::Effect; 5] = [
	crate::analyzer::effects::Effect::State,
	crate::analyzer::effects::Effect::IO,
	crate::analyzer::effects::Effect::FFI,
	crate::analyzer::effects::Effect::Async,
	crate::analyzer::effects::Effect::Eval,
];

/// A registered table: the list variable holding it and its class's fields (name, type, default)
#[derive(Clone)]
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
#[derive(Clone)]
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
	if !crate::warp_parser::mentions(&program, DATABASE_WORD) {
		return program;
	}
	let tables = registrations(&program);
	if tables.is_empty() {
		return program;
	}
	with_lazy_reads(with_tables(with_row_fields(program, &tables), &tables, &tables_file(), &mut vec![]), &tables, None)
}

/// The program's registered tables by their list variable
fn registrations(program: &Node) -> BTreeMap<String, Table> {
	let classes = class_bodies(program);
	let mut tables = BTreeMap::new();
	program.visit(&mut |part| {
		if let Some((variable, table)) = registration(part, &classes) {
			tables.insert(variable, table);
		}
	});
	with_relations(tables)
}

/// Each table's fields of another table's class (foreign keys) and lists of one (one-to-many, pointing back)
fn with_relations(mut tables: BTreeMap<String, Table>) -> BTreeMap<String, Table> {
	let variable_of: HashMap<String, String> =
		tables.iter().map(|(variable, table)| (table.class.clone(), variable.clone())).collect();
	let fields_of: HashMap<String, Vec<(String, String)>> = tables
		.values()
		.map(|table| {
			(
				table.class.clone(),
				table.fields.iter().map(|(name, field_type, _)| (name.clone(), field_type.clone())).collect(),
			)
		})
		.collect();
	for table in tables.values_mut() {
		for (field, field_type, _) in &table.fields {
			if let Some(target) = variable_of.get(class_of(field_type)) {
				table.references.push((field.clone(), target.clone()));
			}
			let element = field_type.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or_default();
			if let Some(target) = variable_of.get(element) {
				let back_field = fields_of[element].iter().find(|(_, back_type)| class_of(back_type) == table.class);
				let (back, optional_back) = (
					back_field.map(|(name, _)| name.clone()),
					back_field.is_some_and(|(_, back_type)| back_type.ends_with(OPTIONAL_MARK)),
				);
				table.members.push(Members { field: field.clone(), table: target.clone(), back, optional_back });
			}
		}
	}
	tables
}

fn tables_file() -> String {
	crate::modules::program_file()
		.map_or_else(|| TABLES_FILE.to_string(), |file| file.with_extension(TABLES_FILE).to_string_lossy().into_owned())
}

/// The tables a program registers, both forms, before database_tables::lower; and the functions its filters' queries call
pub struct Tables {
	tables: BTreeMap<String, Table>,
	functions: Vec<Node>,
	/// the effects of the program's functions, which a filter's query should not have
	effects: Option<crate::analyzer::effects::EffectReport>,
}

pub fn registered(program: &Node) -> Tables {
	let registers = crate::warp_parser::mentions(program, crate::stored_values::STORED_WORD)
		|| crate::warp_parser::mentions(program, DATABASE_WORD);
	let tables = match registers {
		true => registrations(&with_stored_tables(program.clone(), &class_bodies(program))),
		false => BTreeMap::new(),
	};
	let effects = (!tables.is_empty()).then(|| crate::analyzer::effects::EffectReport::of(program));
	Tables { tables, functions: vec![], effects }
}

/// "name" as an SQL identifier
fn quote(name: &str) -> String {
	format!("\"{}\"", name.replace('"', "\"\""))
}

/// A table declaration, `stored people: [Person]` (the table people) or `stored people: [Person] in "people_v2"`
/// (user, 2026-10-08, 2026-10-14)
pub struct StoredTable {
	pub variable: String,
	pub class: String,
	pub table: String,
}

pub fn stored_table(statement: &Node) -> Option<StoredTable> {
	let Node::List(items, _, _) = statement.drop_meta() else {
		return None;
	};
	let [word, declaration] = items.as_slice() else {
		return None;
	};
	let Node::Key(variable, Op::Colon, declared) = declaration.drop_meta() else {
		return None;
	};
	if !word.is_symbol(crate::stored_values::STORED_WORD) {
		return None;
	}
	let variable = variable.drop_meta().name();
	let (list_type, table) = match declared.drop_meta() {
		Node::List(parts, _, _)
			if let [list_type, in_word, table] = parts.as_slice()
				&& in_word.is_symbol(IN_WORD) =>
		{
			(list_type, table.drop_meta().name())
		}
		_ => (declared.as_ref(), variable.clone()),
	};
	match list_type.drop_meta() {
		Node::List(element, Bracket::Square, _) if let [class] = element.as_slice() => {
			Some(StoredTable { variable, class: class.drop_meta().name(), table })
		}
		_ => None,
	}
}

/// The table `database.t`, the store a registration reads its rows from
pub fn table_store(table: &str) -> Node {
	key(symbol(DATABASE_WORD), Op::Dot, symbol(table))
}

/// `stored people: [Person]` of a class of the program as the registration `people: [Person] = database.people`
fn with_stored_tables(node: Node, classes: &HashMap<String, Node>) -> Node {
	match stored_table(&node) {
		Some(stored) if classes.contains_key(&stored.class) => {
			let list_type = Node::List(vec![symbol(&stored.class)], Bracket::Square, Separator::None);
			key(key(symbol(&stored.variable), Op::Colon, list_type), Op::Assign, table_store(&stored.table))
		}
		_ => node.map_children(|child| with_stored_tables(child, classes)),
	}
}

/// Each class's body by name
pub fn class_bodies(program: &Node) -> HashMap<String, Node> {
	let mut bodies = HashMap::new();
	program.visit(&mut |part| {
		if let Node::Type { name, body } = part {
			bodies.insert(name.drop_meta().name(), body.as_ref().clone());
		}
	});
	bodies
}

/// `people: [Person] = database.people` of a class of the program: the variable and its table
fn registration(statement: &Node, classes: &HashMap<String, Node>) -> Option<(String, Table)> {
	let Node::Key(target, Op::Assign, source) = statement.drop_meta() else {
		return None;
	};
	let Node::Key(variable, Op::Colon, list_type) = target.drop_meta() else {
		return None;
	};
	let Node::List(element, Bracket::Square, _) = list_type.drop_meta() else {
		return None;
	};
	let [class] = element.as_slice() else { return None };
	let name = database_table(source)?;
	let class = class.drop_meta().name();
	let body = classes.get(&class)?;
	let fields = crate::class_methods::field_declarations(body);
	let renamed = crate::class_methods::fields_marked(body, RENAMED_MARK)
		.into_iter()
		.map(|(field, old)| (field, old.drop_meta().name()))
		.collect();
	Some((variable.drop_meta().name(), Table { name, class, fields, references: vec![], members: vec![], renamed }))
}

/// The table `database.t` names
fn database_table(source: &Node) -> Option<String> {
	let Node::Key(store, Op::Dot, table) = source.drop_meta() else {
		return None;
	};
	store.is_symbol(DATABASE_WORD).then(|| table.drop_meta().name())
}

/// Each registered class with the field `id: int = 0` unless it has one (0 until the instance has a row), and its
/// one-to-many fields as getters
fn with_row_fields(node: Node, tables: &BTreeMap<String, Table>) -> Node {
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
		let Node::Key(field, Op::Colon, _) = item.drop_meta() else {
			return item;
		};
		let Some(members) = table.members.iter().find(|members| members.field == field.drop_meta().name()) else {
			return item;
		};
		let Some(back) = &members.back else { return item };
		if cfg!(feature = "native") {
			// the rows pointing back, selected: the other table is not loaded
			let condition = (CONDITION_PLACEHOLDER, Node::Text(format!("{} IS ?", quote(back))));
			return generated(
				&format!("{field} := {{ {FOUND}({CONDITION_PLACEHOLDER}, [{ID_FIELD}]) }}", field = members.field),
				lazy_names(&members.table).into_iter().chain([condition]),
			);
		}
		generated(
			&format!(
				"{field} := {{ [{MEMBER} for {MEMBER} in {LOAD}() if {key} == {ID_FIELD}] }}",
				key = key_id(MEMBER, back, members.optional_back),
				field = members.field
			),
			lazy_names(&members.table),
		)
	};
	match body {
		Node::List(items, Bracket::Curly, separator) if !table.members.is_empty() => {
			Node::List(items.into_iter().map(getter).collect(), Bracket::Curly, separator)
		}
		body => body,
	}
}

/// The named classes with their row's `id: int = 0`: a page leaving their table on the server still reads `todo.id` of
/// the rows it is sent (lowering/web/serve.rs)
pub(crate) fn with_row_ids(node: Node, classes: &[String]) -> Node {
	match node {
		Node::Type { name, body } if classes.contains(&name.drop_meta().name()) => {
			Node::Type { name, body: Box::new(with_id(*body)) }
		}
		other => other.map_children(|child| with_row_ids(child, classes)),
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
		// `{name: text age: int = 0}`: fields side by side
		Node::List(mut items, Bracket::Curly, _) if items.len() == 1 || items.iter().all(is_typed_field) => {
			items.push(id);
			Node::List(items, Bracket::Curly, Separator::Semicolon)
		}
		// `{int x}`: one field of words
		Node::List(words, Bracket::Curly, separator) => {
			Node::List(vec![Node::List(words, Bracket::None, separator), id], Bracket::Curly, Separator::Semicolon)
		}
		body => Node::List(vec![body, id], Bracket::Curly, Separator::Semicolon),
	}
}

/// `age: int`, `age: int = 0`
fn is_typed_field(item: &Node) -> bool {
	match item.drop_meta() {
		Node::Key(_, Op::Colon, _) => true,
		Node::Key(field, Op::Assign | Op::Define, _) => matches!(field.drop_meta(), Node::Key(_, Op::Colon, _)),
		_ => false,
	}
}

fn has_id(body: &Node) -> bool {
	crate::class_methods::field_declarations(body).iter().any(|(name, _, _)| name == ID_FIELD)
}

/// The registrations, inserts and field changes of the tables as their table calls, in every block; `open` the tables
/// registered so far, as a foreign key reads the rows of its table
fn with_tables(node: Node, tables: &BTreeMap<String, Table>, file: &str, open: &mut Vec<String>) -> Node {
	if let Some(lowered) = saved(&node, tables, file).or_else(|| added_to_members(&node, tables, file)) {
		return lowered;
	}
	if let Some(wrapped) = in_transaction(&node, file, open) {
		return with_tables(wrapped, tables, file, open);
	}
	match node {
		Node::List(statements, bracket, separator @ (Separator::Semicolon | Separator::Newline)) => {
			let statements =
				statements.into_iter().flat_map(|statement| table_statements(statement, tables, file, open)).collect();
			Node::List(statements, bracket, separator)
		}
		// a block of one statement: `if count(teams) == 0 { teams.add(red) }`
		Node::List(statements, Bracket::Curly, separator) if statements.len() == 1 => {
			let statements: Vec<Node> =
				statements.into_iter().flat_map(|statement| table_statements(statement, tables, file, open)).collect();
			let separator = if statements.len() == 1 { separator } else { Separator::Semicolon };
			Node::List(statements, Bracket::Curly, separator)
		}
		other => other.map_children(|child| with_tables(child, tables, file, open)),
	}
}

fn table_statements(
	statement: Node,
	tables: &BTreeMap<String, Table>,
	file: &str,
	open: &mut Vec<String>,
) -> Vec<Node> {
	if let Some(lowered) = opened(&statement, tables, file, open).or_else(|| inserted(&statement, tables, file)) {
		return lowered;
	}
	if let Some(bound) = with_bound_element(&statement, tables) {
		return bound.into_iter().flat_map(|statement| table_statements(statement, tables, file, open)).collect();
	}
	let updates = written_through(&statement, tables, file);
	[vec![with_tables(statement, tables, file, open)], updates].concat()
}

/// `people#1.age = 5`, `(people where …)#1.age += 1`, `page#1.age = 5`: a column of an element of a table changed in place, as the
/// element bound first, `people·element = people#1; people·element.age = 5`, whose change written_through writes to
/// its row
fn with_bound_element(statement: &Node, tables: &BTreeMap<String, Table>) -> Option<Vec<Node>> {
	let Node::Key(target, op, value) = statement.drop_meta() else {
		return None;
	};
	if *op != Op::Assign && !op.is_compound_assign() {
		return None;
	}
	let Node::Key(instance, Op::Dot, field) = target.drop_meta() else {
		return None;
	};
	if matches!(instance.drop_meta(), Node::Symbol(_)) {
		return None;
	}
	// an element of another list, `page#1.name = …`, may be a row of any table with the column: written_through asks
	let has_column = |name: &&String| columns_of(&tables[*name]).contains(&field.drop_meta().name());
	let variable = match tables.keys().find(|name| instance.mentions_any(&[name.as_str()])) {
		Some(mentioned) => Some(mentioned).filter(has_column)?,
		None => tables.keys().find(has_column)?,
	};
	let element = Node::Symbol(lazy_name(variable, "element"));
	let changed = Node::Key(Box::new(Node::Key(Box::new(element.clone()), Op::Dot, field.clone())), *op, value.clone());
	Some(vec![Node::Key(Box::new(element), Op::Assign, instance.clone()), changed])
}

/// `people: [Person] = database.people` as the list of the table's rows, each an instance with its id; then the
/// one-to-many lists of the open tables its rows point back to. A plain `people = database.people` of a registered
/// people opens it again (a served route reading it at each request)
fn opened(statement: &Node, tables: &BTreeMap<String, Table>, file: &str, open: &mut Vec<String>) -> Option<Vec<Node>> {
	let Node::Key(target, Op::Assign, source) = statement.drop_meta() else {
		return None;
	};
	let declared = matches!(target.drop_meta(), Node::Key(_, Op::Colon, _));
	let variable = match target.drop_meta() {
		Node::Key(variable, Op::Colon, _) => variable.drop_meta(),
		variable @ Node::Symbol(_) => variable,
		_ => return None,
	};
	let variable = variable.name();
	let table = tables.get(&variable)?;
	database_table(source)?;
	if let Some(problem) = opening_problem(table, &variable, open) {
		return Some(vec![crate::errors::diagnostic::Diagnostic::at(statement, problem).into_error()]);
	}
	open.push(variable.clone());
	if !declared {
		// `people = database.people` again (a served route at each request): the next read loads the rows anew; an
		// assignment still, so a block of it and a value stays a block (`{ p = …; p#1 }`), no list
		let reset = called(lazy_name(&variable, "reset"), None);
		return Some(vec![key(target.drop_meta().clone(), Op::Assign, reset)]);
	}
	let empty = parse("[]");
	Some(
		[
			vec![key(target.drop_meta().clone(), Op::Assign, empty)],
			generated(&table_program(table, &variable, file), table_placeholders(table, &variable, file)).children(),
		]
		.concat(),
	)
}

/// Why the table cannot open yet: a table its keys point to is not open, or a list of rows has no key pointing back
fn opening_problem(table: &Table, variable: &str, open: &[String]) -> Option<String> {
	if let Some((field, unopened)) = table.references.iter().find(|(_, referenced)| !open.contains(referenced)) {
		return Some(format!("register {unopened} before {variable}: {}.{field} points to its rows", table.class));
	}
	table.members.iter().find(|members| members.back.is_none()).map(|members| {
		format!(
			"{}.{} lists rows of {}, whose class has no field of class {} pointing back",
			table.class, members.field, members.table, table.class
		)
	})
}

/// The table's columns for the store: a column is [name type default] and its old name when renamed (ø when not); a
/// unit field's adds its quantity and SI amount per unit, which the browser's store has no unit table for
/// (`[distance km ø ø m 1000]`)
fn table_schema(table: &Table) -> Node {
	Node::List(
		column_fields(table)
			.into_iter()
			.map(|(name, field_type, default)| {
				let old_name =
					table.renamed.iter().find(|(field, _)| *field == name).map(|(_, old)| Node::Text(old.clone()));
				let unit = crate::numbers::units::static_units::unit_type(class_of(&field_type))
					.map(|(quantity, per_unit)| [Node::Text(quantity), crate::node::float(per_unit)]);
				let old_name = if unit.is_some() { Some(old_name.unwrap_or(Node::Empty)) } else { old_name };
				let column = [Node::Text(name), Node::Text(field_type), default.unwrap_or(Node::Empty)]
					.into_iter()
					.chain(old_name)
					.chain(unit.into_iter().flatten());
				Node::List(column.collect(), Bracket::Square, Separator::Space)
			})
			.collect(),
		Bracket::Square,
		Separator::Space,
	)
}

/// The store calls of the table program, its placeholders, and the names of its functions
fn table_placeholders(table: &Table, variable: &str, file: &str) -> impl Iterator<Item = (&'static str, Node)> {
	let schema = table_schema(table);
	let table_call_with = |member: &str, extra: &str| {
		generated(
			&format!("std_io(\"table\", {member:?}, [{:?}, {SCHEMA_PLACEHOLDER}, {file:?}{extra}])", table.name),
			[(SCHEMA_PLACEHOLDER, schema.clone())],
		)
	};
	let table_call = |member: &str| table_call_with(member, "");
	[
		(READ_PLACEHOLDER, table_call("migrate")),
		(ROWS_PLACEHOLDER, table_call("rows")),
		(COUNT_PLACEHOLDER, table_call("count")),
		(ROW_PLACEHOLDER, table_call_with("page", &format!(", {POSITION}, 1"))),
		(PAGE_PLACEHOLDER, table_call_with("page", &format!(", {POSITION}, {PAGE_SIZE}"))),
		(
			SLICE_PLACEHOLDER,
			table_call_with("page", &format!(", {FIRST} + 1, (if {UNTIL} > {FIRST} then {UNTIL} - {FIRST} else 0)")),
		),
		(IN_PLACEHOLDER, Node::Text(format!("{} IN (", quote(ID_FIELD)))),
		(SELECT_PLACEHOLDER, table_call_with("select", &format!(", {CONDITION}, {CONDITION_VALUES}"))),
	]
	.into_iter()
	.chain(lazy_names(variable))
}

/// The fields whose key must find its row: not optional ones
fn required_references(table: &Table) -> Vec<&str> {
	table.references.iter().map(|(field, _)| field.as_str()).filter(|field| !table.is_optional(field)).collect()
}

/// Whether the row's key `field` finds its row (`present`) or not; a required key without its row (deleted, or the 0
/// of a column added for it) leaves its row out, reported
fn reference_found(table: &Table, field: &str, present: bool) -> String {
	match is_own_reference(table, field) {
		true => format!(
			"count({}) {} 0",
			matching_rows(table, field, &column_cell(table, ROW, field)),
			if present { ">" } else { "==" }
		),
		false => format!(
			"({}) {} ø",
			referenced_instance(table, field, &column_cell(table, ROW, field)),
			if present { "!=" } else { "==" }
		),
	}
}

/// The warnings for rows whose required key finds no row
fn missing_reference_checks(table: &Table, variable: &str, required: &[&str]) -> String {
	if required.is_empty() {
		return String::new();
	}
	let class_of_field = |field: &str| {
		table
			.fields
			.iter()
			.find(|(name, _, _)| name == field)
			.map_or(String::new(), |(_, field_type, _)| class_of(field_type).to_string())
	};
	let warnings: String = required.iter().map(|field| format!(
		"if {missing} {{ warning(\"{variable} row \" + {ROW}#1 + \": {field} \" + {cell} + \" is no row of {target}; the row is left out (declare {field}: {class}? to keep it, with ø)\") }}\n",
		missing = reference_found(table, field, false), cell = column_cell(table, ROW, field), target = table.reference(field).unwrap_or_default(),
		class = class_of_field(field))).collect();
	format!("for {ROW} in {ROWS} {{\n{warnings}}}\n")
}

/// The condition keeping only the rows whose required keys find their rows
fn kept_condition(table: &Table, required: &[&str]) -> String {
	match required.is_empty() {
		true => String::new(),
		false => format!(
			" if {}",
			required.iter().map(|field| reference_found(table, field, true)).collect::<Vec<_>>().join(" and ")
		),
	}
}

/// Natively the rows read before loading find their keys' rows together, `id IN (…)` (card orm-batched); the browser
/// has no SQL and loads the other table
fn batched_fetches(table: &Table) -> String {
	match cfg!(feature = "native") {
		true => table
			.references
			.iter()
			.filter(|(field, _)| !is_own_reference(table, field))
			.map(|(field, target)| {
				format!("{target}.{FETCHED_WORD}([{} for {ROW} in {ROWS}])\n", column_cell(table, ROW, field))
			})
			.collect(),
		false => String::new(),
	}
}

/// The count, element, streamed and sliced reads before loading; the rows a required key leaves out are counted and
/// positioned only by loading
fn unloaded_reads(has_required: bool, fetches: &str, class: &str) -> [String; 4] {
	match has_required {
		false => [
			COUNT_PLACEHOLDER.to_string(),
			format!(
				"{{
{ROWS} = {ROW_PLACEHOLDER}
{fetches}if not {ROWS} then {LOAD}()#{POSITION} else {KEPT}({ROWS}#1)
}}"
			),
			format!(
				"{{
if {POSITION} < {START} or {POSITION} >= {START} + count({PAGE}) {{
{ROWS} = {PAGE_PLACEHOLDER}
{fetches}{PAGE} = [{KEPT}({ROW}) for {ROW} in {ROWS}]
{START} = {POSITION}
}}
{PAGE}#({POSITION} - {START} + 1)
}}"
			),
			format!(
				"{{
{ROWS} = {SLICE_PLACEHOLDER}
{fetches}{SLICE_ROWS}: [{class}] = [{KEPT}({ROW}) for {ROW} in {ROWS}]
{SLICE_ROWS}
}}"
			),
		],
		true => [
			format!("count({LOAD}())"),
			format!("{LOAD}()#{POSITION}"),
			format!("{LOAD}()#{POSITION}"),
			format!("slice({LOAD}(), {FIRST}, {UNTIL})"),
		],
	}
}

/// The warp program of an opened table: its lazy loading, identity map, writes, queries and restore
fn table_program(table: &Table, variable: &str, file: &str) -> String {
	let arguments = constructor_arguments(table, ROW);
	let required = required_references(table);
	let fetches = batched_fetches(table);
	let checks = missing_reference_checks(table, variable, &required);
	let kept = kept_condition(table, &required);
	let class = &table.class;
	// the identity map: the instance of each id met, a hash table keyed by the id (card int-map)
	let known = format!("{ROW}#1 in {MET}");
	let constructed = format!("{class}({})", arguments.join(", "));
	let instance = format!("(if {known} then {MET}[{ROW}#1] else {constructed})");
	let [unloaded_count, unloaded_element, unloaded_streamed, unloaded_sliced] =
		unloaded_reads(!required.is_empty(), &fetches, class);
	format!(
		"{LOADED} = no
{MET} = {{}}
{PAGE}: [{class}] = []
{START} = 0
{READ_PLACEHOLDER}
{LOAD}() := {{
global {variable}
global {LOADED}
global {MET}
if not {LOADED} {{
{ROWS} = {ROWS_PLACEHOLDER}
{fetches}{checks}{variable} = [{instance} for {ROW} in {ROWS}{kept}]
for {MADE} in {variable} {{ {MET}[{MADE}.{ID_FIELD}] = {MADE} }}
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
if {LOADED} {{ {variable}.add({ADDED}) }}
{MET}[{ADDED}.{ID_FIELD}] = {ADDED}
{ADDED}
}}
{REMOVING}({REMOVED}) := {{
global {variable}
global {MET}
std_io(\"table\", \"delete\", [{name:?}, {REMOVED}.{ID_FIELD}, {file:?}])
{variable} = [{ROW} for {ROW} in {variable} if {ROW}.{ID_FIELD} != {REMOVED}.{ID_FIELD}]
{MET}.remove({REMOVED}.{ID_FIELD})
{REMOVED}
}}
{KEPT}({ROW}) := {{
global {MET}
if {known} then {MET}[{ROW}#1] else {{
{MADE} = {constructed}
{MET}[{MADE}.{ID_FIELD}] = {MADE}
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
{SLICED}({FROM}, {UNTIL}) := {{
global {variable}
global {LOADED}
{FIRST} = if {FROM} < 0 then 0 else {FROM}
if {LOADED} then slice({variable}, {FIRST}, {UNTIL}) else {unloaded_sliced}
}}
{RESET}() := {{
global {LOADED}
global {MET}
global {PAGE}
{LOADED} = no
{MET} = {{}}
{PAGE} = []
[]
}}
{FOUND}({CONDITION}, {CONDITION_VALUES}) := {{
global {variable}
global {LOADED}
global {MET}
{ROWS} = {SELECT_PLACEHOLDER}
{fetches}if {loaded_found} {{
{LOAD}()
[{MET}[{ROW}#1] for {ROW} in {ROWS} if {ROW}#1 in {MET}]
}} else [{KEPT}({ROW}) for {ROW} in {ROWS}]
}}
{OF}({KEY}) := {{
global {LOADED}
global {MET}
{of_unmet}
if {KEY} in {MET} then {MET}[{KEY}] else ø
}}
{FETCHED}({KEYS}) := {{
global {LOADED}
global {MET}
if not {LOADED} {{
{MISSING} = [{KEY} for {KEY} in {KEYS} if {KEY} and not ({KEY} in {MET})]
if count({MISSING}) > {MOST_FETCHED} {{ {LOAD}() }} else if {MISSING} {{ {FOUND}({IN_PLACEHOLDER} + ((count({MISSING}) - 1) times \"?, \") + \"?)\", {MISSING}) }}
}}
[]
}}
{RESTORE}() := {{
global {variable}
global {LOADED}
global {MET}
global {PAGE}
{ROWS} = {ROWS_PLACEHOLDER}
{KNOWN} = {{}}
for {ROW} in {ROWS} {{ {KNOWN}[{ROW}#1] = {ROW} }}
{restore_met}
for {KEY} in keys({MET}) {{ if {MET}[{KEY}].{ID_FIELD} == 0 {{ {MET}.remove({KEY}) }} }}
if {LOADED} {{
{restore_loaded}
for {MADE} in {variable} {{ if {MADE}.{ID_FIELD} != 0 {{ {MET}[{MADE}.{ID_FIELD}] = {MADE} }} }}
}}
{LOADED} = no
{PAGE} = []
[]
}}",
		name = table.name,
		restore_met = restored_rows(table, &arguments, &format!("values({MET})")),
		restore_loaded = restored_rows(table, &arguments, variable),
		loaded_found = if required.is_empty() { LOADED } else { "yes" },
		of_unmet = match cfg!(feature = "native") {
			true => format!("if not {LOADED} and not ({KEY} in {MET}) {{ {FETCHED}([{KEY}]) }}"),
			false => format!("if not {LOADED} {{ {LOAD}() }}"),
		}
	)
}
