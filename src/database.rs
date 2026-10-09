//! The tables of the program's registered classes (lowering/database_tables.rs, notes/orm.md) in SQLite, through the
//! system's libsqlite3 (loaded like any FFI library, ffi/link.rs): `std_io("table", member, …)` answered natively.
//! A connection per file and thread; inline code's tables (database_tables::TABLES_FILE) live in memory per thread.

use crate::node::{Bracket, Node, Separator};
use crate::Number;
use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};

const SQLITE_LIBRARY: &str = "sqlite3";
const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;
const SQLITE_INTEGER: c_int = 1;
const SQLITE_FLOAT: c_int = 2;
const SQLITE_TEXT: c_int = 3;
/// sqlite3_bind_text copies the text (SQLITE_TRANSIENT)
const SQLITE_TRANSIENT: isize = -1;
const SQLITE_UTF8: c_int = 1;
const IN_MEMORY: &str = ":memory:";
const ID_COLUMN: &str = "id";
/// column types each holding every value of the ones before it: a column converts forward without loss
const LOSSLESS_ORDER: [&str; 3] = ["INTEGER", "REAL", "TEXT"];
const NUMBER_COLUMNS: [&str; 2] = ["INTEGER", "REAL"];
/// The column of a unit field (`NUMERIC km`): NUMERIC keeps a whole SI amount an integer, as the program holds it
const UNIT_COLUMN: &str = "NUMERIC";
/// The SQL function a filter's query calls a warp function through: `warp_call('function', arguments…)`
const WARP_CALL: &str = "warp_call";
/// How long a write waits for another connection's (another program or thread) before "database is locked"
const BUSY_MILLISECONDS: c_int = 5000;
/// A folder set by cargo's test runs (.cargo/config.toml): each test thread keeps a sample's tables in its own copy there
const TEST_TABLES: &str = "WARP_TEST_TABLES";
/// The folder of the samples, which several tests run at once
const SAMPLES_FOLDER: &str = "samples";

/// The program's function by name, called with the list of its arguments (src/host.rs calls back into the instance)
pub type Callback<'a> = dyn FnMut(&str, &Node) -> Result<Node, String> + 'a;
type SqlFunction = unsafe extern "C" fn(Handle, c_int, *mut Handle);

type Handle = *mut c_void;

/// The functions of the SQLite C API the tables use
struct Sqlite {
	open: unsafe extern "C" fn(*const c_char, *mut Handle) -> c_int,
	busy_timeout: unsafe extern "C" fn(Handle, c_int) -> c_int,
	errmsg: unsafe extern "C" fn(Handle) -> *const c_char,
	prepare: unsafe extern "C" fn(Handle, *const c_char, c_int, *mut Handle, *mut *const c_char) -> c_int,
	bind_int64: unsafe extern "C" fn(Handle, c_int, i64) -> c_int,
	bind_double: unsafe extern "C" fn(Handle, c_int, f64) -> c_int,
	bind_text: unsafe extern "C" fn(Handle, c_int, *const c_char, c_int, isize) -> c_int,
	bind_null: unsafe extern "C" fn(Handle, c_int) -> c_int,
	step: unsafe extern "C" fn(Handle) -> c_int,
	column_count: unsafe extern "C" fn(Handle) -> c_int,
	column_type: unsafe extern "C" fn(Handle, c_int) -> c_int,
	column_int64: unsafe extern "C" fn(Handle, c_int) -> i64,
	column_double: unsafe extern "C" fn(Handle, c_int) -> f64,
	column_text: unsafe extern "C" fn(Handle, c_int) -> *const c_char,
	finalize: unsafe extern "C" fn(Handle) -> c_int,
	last_insert_rowid: unsafe extern "C" fn(Handle) -> i64,
	create_function: unsafe extern "C" fn(Handle, *const c_char, c_int, c_int, *mut c_void, Option<SqlFunction>, Option<SqlFunction>, Option<unsafe extern "C" fn(Handle)>, Option<unsafe extern "C" fn(*mut c_void)>) -> c_int,
	user_data: unsafe extern "C" fn(Handle) -> *mut c_void,
	value_type: unsafe extern "C" fn(Handle) -> c_int,
	value_int64: unsafe extern "C" fn(Handle) -> i64,
	value_double: unsafe extern "C" fn(Handle) -> f64,
	value_text: unsafe extern "C" fn(Handle) -> *const c_char,
	result_int64: unsafe extern "C" fn(Handle, i64),
	result_double: unsafe extern "C" fn(Handle, f64),
	result_text: unsafe extern "C" fn(Handle, *const c_char, c_int, isize),
	result_null: unsafe extern "C" fn(Handle),
	result_error: unsafe extern "C" fn(Handle, *const c_char, c_int),
}

static SQLITE: std::sync::OnceLock<Result<Sqlite, String>> = std::sync::OnceLock::new();

fn sqlite() -> Result<&'static Sqlite, String> {
	SQLITE.get_or_init(load_sqlite).as_ref().map_err(Clone::clone)
}

fn load_sqlite() -> Result<Sqlite, String> {
	let library = crate::ffi::get_or_load_library(SQLITE_LIBRARY).ok_or_else(|| format!("no lib{SQLITE_LIBRARY} found: database tables need SQLite"))?;
	macro_rules! function {
		($name:literal) => {
			// the library stays loaded for the process (ffi/link.rs keeps it), so its function pointers stay valid
			*unsafe { library.get(concat!($name, "\0").as_bytes()) }.map_err(|problem| format!("{}: {problem}", $name))?
		};
	}
	Ok(Sqlite {
		open: function!("sqlite3_open"),
		busy_timeout: function!("sqlite3_busy_timeout"),
		errmsg: function!("sqlite3_errmsg"),
		prepare: function!("sqlite3_prepare_v2"),
		bind_int64: function!("sqlite3_bind_int64"),
		bind_double: function!("sqlite3_bind_double"),
		bind_text: function!("sqlite3_bind_text"),
		bind_null: function!("sqlite3_bind_null"),
		step: function!("sqlite3_step"),
		column_count: function!("sqlite3_column_count"),
		column_type: function!("sqlite3_column_type"),
		column_int64: function!("sqlite3_column_int64"),
		column_double: function!("sqlite3_column_double"),
		column_text: function!("sqlite3_column_text"),
		finalize: function!("sqlite3_finalize"),
		last_insert_rowid: function!("sqlite3_last_insert_rowid"),
		create_function: function!("sqlite3_create_function_v2"),
		user_data: function!("sqlite3_user_data"),
		value_type: function!("sqlite3_value_type"),
		value_int64: function!("sqlite3_value_int64"),
		value_double: function!("sqlite3_value_double"),
		value_text: function!("sqlite3_value_text"),
		result_int64: function!("sqlite3_result_int64"),
		result_double: function!("sqlite3_result_double"),
		result_text: function!("sqlite3_result_text"),
		result_null: function!("sqlite3_result_null"),
		result_error: function!("sqlite3_result_error"),
	})
}

thread_local! {
	/// The open connections by file, kept while the thread runs (inline code's tables live as long as its connection)
	static CONNECTIONS: std::cell::RefCell<HashMap<String, usize>> = Default::default();
	/// The rows tables gave whole while the thread runs (a lazy table loads none until its list is read)
	static ROWS_READ: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many rows opening tables read on this thread so far
pub fn rows_read() -> usize {
	ROWS_READ.get()
}

/// `std_io("table", member, arguments)`: open (create or migrate, give the rows), migrate and rows (its two halves), page (from a
/// position), count, insert
/// (give the id), update
pub fn call(member: &str, arguments: &[Node]) -> Result<Node, String> {
	let text = |node: &Node| match node.drop_meta() {
		Node::Text(text) => Ok(text.clone()),
		other => Err(format!("needs a text, got {}", other.serialize().trim())),
	};
	match (member, arguments) {
		("open", [table, schema, file]) => opened(&text(table)?, &schema.children(), &text(file)?),
		("rows", [table, schema, file]) => {
			let columns = schema.children().iter().map(column_of).collect::<Result<Vec<_>, _>>()?;
			selected(connection(&text(file)?)?, &text(table)?, &columns, None)
		}
		("page", [table, schema, file, start, size]) => {
			let columns = schema.children().iter().map(column_of).collect::<Result<Vec<_>, _>>()?;
			selected(connection(&text(file)?)?, &text(table)?, &columns, Some([start, size]))
		}
		("migrate", [table, schema, file]) => migrated(connection(&text(file)?)?, &text(table)?, &schema.children()).map(|_| Node::Empty),
		("count", [table, _schema, file]) => {
			let counted = rows(connection(&text(file)?)?, &format!("SELECT COUNT(*) FROM {}", quote(&text(table)?)), &[])?;
			Ok(counted.into_iter().flatten().next().unwrap_or(Node::int(0)))
		}
		("insert", [table, columns, values, file]) => {
			let (table, columns) = (text(table)?, columns.children().iter().map(text).collect::<Result<Vec<_>, _>>()?);
			let placeholders = vec!["?"; columns.len()].join(", ");
			let database = connection(&text(file)?)?;
			let quoted: Vec<String> = columns.iter().map(|column| quote(column)).collect();
			rows(database, &format!("INSERT INTO {} ({}) VALUES ({placeholders})", quote(&table), quoted.join(", ")), &values.children())?;
			Ok(Node::int(unsafe { (sqlite()?.last_insert_rowid)(database) }))
		}
		("update", [table, id, column, value, file]) => {
			let sql = format!("UPDATE {} SET {} = ? WHERE {ID_COLUMN} = ?", quote(&text(table)?), quote(&text(column)?));
			rows(connection(&text(file)?)?, &sql, &[value.clone(), id.clone()]).map(|_| Node::Empty)
		}
		_ => Err(format!("no such word of {} arguments", arguments.len())),
	}
}

/// `std_io("table", "select", [table, condition, parameters, file])`: the ids of the rows the SQL condition keeps, in
/// order; the condition's `warp_call('f', …)` calls the program's function f, registered for this query only
pub fn select(arguments: &[Node], callback: &mut Callback) -> Result<Node, String> {
	let [table, condition, parameters, file] = arguments else { return Err(format!("select needs 4 arguments, got {}", arguments.len())) };
	let database = connection(&file.drop_meta().name())?;
	let sql = format!("SELECT {ID_COLUMN} FROM {} WHERE {} ORDER BY {ID_COLUMN}", quote(&table.drop_meta().name()), condition.drop_meta().name());
	let mut call = WarpCall { callback, failure: None };
	register_warp_call(database, Some((&mut call as *mut WarpCall).cast()))?;
	let found = rows(database, &sql, &parameters.children());
	register_warp_call(database, None)?;
	if let Some(failure) = call.failure {
		return Err(failure);
	}
	Ok(list(found?.into_iter().flatten().collect()))
}

/// What warp_call reaches during one query: the program, and its first failure
struct WarpCall<'a, 'b> {
	callback: &'a mut Callback<'b>,
	failure: Option<String>,
}

/// warp_call registered on the connection with the query's WarpCall, or removed (None)
fn register_warp_call(database: Handle, call: Option<*mut c_void>) -> Result<(), String> {
	let sqlite = sqlite()?;
	let name = CString::new(WARP_CALL).map_err(|problem| problem.to_string())?;
	let function = call.is_some().then_some(warp_call as SqlFunction);
	let code = unsafe { (sqlite.create_function)(database, name.as_ptr(), -1, SQLITE_UTF8, call.unwrap_or(std::ptr::null_mut()), function, None, None, None) };
	match code {
		SQLITE_OK => Ok(()),
		_ => Err(format!("{WARP_CALL}: {}", message(sqlite, database))),
	}
}

/// `warp_call('f', a, b…)` in a query: f([a, b…]) of the program, its value given back to SQLite
unsafe extern "C" fn warp_call(context: Handle, count: c_int, values: *mut Handle) {
	let Ok(sqlite) = sqlite() else { return };
	let call = unsafe { &mut *(sqlite.user_data)(context).cast::<WarpCall>() };
	let arguments: Vec<Node> = (0..count as usize).map(|index| unsafe { value_of(sqlite, *values.add(index)) }).collect();
	let Some((function, arguments)) = arguments.split_first() else { return };
	let answer = (call.callback)(&function.name(), &list(arguments.to_vec())).and_then(|answer| unsafe { give(sqlite, context, &answer) });
	if let Err(failure) = answer {
		let text = CString::new(failure.replace('\0', "")).unwrap_or_default();
		unsafe { (sqlite.result_error)(context, text.as_ptr(), -1) };
		call.failure.get_or_insert(failure);
	}
}

unsafe fn value_of(sqlite: &Sqlite, value: Handle) -> Node {
	unsafe {
		match (sqlite.value_type)(value) {
			SQLITE_INTEGER => Node::int((sqlite.value_int64)(value)),
			SQLITE_FLOAT => Node::float((sqlite.value_double)(value)),
			SQLITE_TEXT => Node::Text(CStr::from_ptr((sqlite.value_text)(value)).to_string_lossy().into_owned()),
			_ => Node::Empty,
		}
	}
}

/// A warp value as the result of an SQL function: a number, a text, a truth (1 or 0) or ø (NULL)
unsafe fn give(sqlite: &Sqlite, context: Handle, value: &Node) -> Result<(), String> {
	unsafe {
		match value.drop_meta() {
			Node::Empty => (sqlite.result_null)(context),
			Node::Number(Number::Int(number)) => (sqlite.result_int64)(context, *number),
			Node::Number(Number::Float(number)) => (sqlite.result_double)(context, *number),
			Node::True => (sqlite.result_int64)(context, 1),
			Node::False => (sqlite.result_int64)(context, 0),
			Node::Text(_) | Node::Char(_) => {
				let text = CString::new(value.name()).map_err(|problem| problem.to_string())?;
				(sqlite.result_text)(context, text.as_ptr(), -1, SQLITE_TRANSIENT)
			}
			other => return Err(format!("a filter's function gave {}, which a query cannot compare", other.serialize().trim())),
		}
	}
	Ok(())
}

/// The table's rows `[id, columns…]`, the table first created or migrated to the class's fields `[name, type, default]`
/// (and the column's old name when the field is marked `@was(old)`)
fn opened(table: &str, schema: &[Node], file: &str) -> Result<Node, String> {
	let database = connection(file)?;
	let columns = migrated(database, table, schema)?;
	selected(database, table, &columns, None)
}

/// The table's rows `[id, columns…]` in id order, or only a page of them: its start position (counted from 1) and size
fn selected(database: Handle, table: &str, columns: &[(String, String, String)], page: Option<[&Node; 2]>) -> Result<Node, String> {
	let selected: Vec<String> = std::iter::once(ID_COLUMN.to_string()).chain(columns.iter().map(|(name, _, _)| quote(name))).collect();
	let (limit, parameters) = match page {
		Some([start, size]) => (" LIMIT ? OFFSET ? - 1", vec![size.clone(), start.clone()]),
		None => ("", vec![]),
	};
	let rows = rows(database, &format!("SELECT {} FROM {} ORDER BY {ID_COLUMN}{limit}", selected.join(", "), quote(table)), &parameters)?;
	ROWS_READ.set(ROWS_READ.get() + rows.len());
	Ok(list(rows.into_iter().map(list).collect()))
}

/// The table created or migrated to the class's fields; their columns
fn migrated(database: Handle, table: &str, schema: &[Node]) -> Result<Vec<(String, String, String)>, String> {
	let columns = schema.iter().map(column_of).collect::<Result<Vec<_>, _>>()?;
	let definitions: Vec<String> = columns.iter().map(|(name, column_type, _)| format!("{} {column_type}", quote(name))).collect();
	let quoted_table = quote(table);
	rows(database, &format!("CREATE TABLE IF NOT EXISTS {quoted_table} ({ID_COLUMN} INTEGER PRIMARY KEY{})", definitions.iter().map(|definition| format!(", {definition}")).collect::<String>()), &[])?;
	let has_column = |stored: &[(String, String)], name: &str| stored.iter().any(|(stored_name, _)| stored_name == name);
	let before_renames = stored_columns(database, &quoted_table)?;
	for (name, old_name) in schema.iter().filter_map(renamed_column) {
		if has_column(&before_renames, &old_name) && !has_column(&before_renames, &name) {
			rows(database, &format!("ALTER TABLE {quoted_table} RENAME COLUMN {} TO {}", quote(&old_name), quote(&name)), &[])?;
		}
	}
	let stored = stored_columns(database, &quoted_table)?;
	for (name, column_type, default) in &columns {
		match stored.iter().find(|(stored_name, _)| stored_name == name) {
			None => { rows(database, &format!("ALTER TABLE {quoted_table} ADD COLUMN {} {column_type} DEFAULT {default}", quote(name)), &[])?; }
			Some((_, stored_type)) if !stored_type.eq_ignore_ascii_case(column_type) => converted(database, table, name, stored_type, column_type)?,
			Some(_) => {}
		}
	}
	for (name, _) in stored.iter().filter(|(name, _)| name != ID_COLUMN && !columns.iter().any(|(column, _, _)| column == name)) {
		crate::diagnostic::report_runtime_warning(&format!("the table {table} keeps its column {name}, which the class no longer has (its data is kept)"));
	}
	Ok(columns)
}

/// Each column's name and SQL type as the table holds them
fn stored_columns(database: Handle, quoted_table: &str) -> Result<Vec<(String, String)>, String> {
	Ok(rows(database, &format!("PRAGMA table_info({quoted_table})"), &[])?.into_iter()
		.map(|row| (row[1].drop_meta().name(), row[2].drop_meta().name())).collect())
}

/// A field `[name, type, default, old name]` of a renamed column: the name and the old one
fn renamed_column(field: &Node) -> Option<(String, String)> {
	let parts = field.children();
	let [name, _, _, old_name] = parts.as_slice() else { return None };
	Some((name.drop_meta().name(), old_name.drop_meta().name()))
}

/// The column retyped in place when no value loses anything (INTEGER → REAL → TEXT, km → m), else a loud error. A fresh
/// column with the new type: SQLite's column affinity would turn the converted values back
fn converted(database: Handle, table: &str, name: &str, stored_type: &str, column_type: &str) -> Result<(), String> {
	let factor = conversion_factor(table, name, stored_type, column_type)?;
	let (quoted_table, column, kept) = (quote(table), quote(name), quote(&format!("{name} before {column_type}")));
	let value = match factor == 1.0 {
		true => format!("CAST({kept} AS {column_type})"),
		false => format!("CAST({kept} AS REAL) * {factor:?}"),
	};
	rows(database, "SAVEPOINT converted", &[])?;
	let steps = [
		format!("ALTER TABLE {quoted_table} RENAME COLUMN {column} TO {kept}"),
		format!("ALTER TABLE {quoted_table} ADD COLUMN {column} {column_type}"),
		format!("UPDATE {quoted_table} SET {column} = {value}"),
		format!("ALTER TABLE {quoted_table} DROP COLUMN {kept}"),
	];
	let done = steps.iter().try_for_each(|statement| rows(database, statement, &[]).map(drop));
	if done.is_err() {
		rows(database, "ROLLBACK TO converted", &[])?;
	}
	rows(database, "RELEASE converted", &[])?;
	done.map_err(|failure| format!("converting the column {table}.{name} from {stored_type} to {column_type} failed: {failure}"))
}

/// The factor a column's values take to the class's field type, or the loud error of a change losing data. A unit
/// column (`NUMERIC km`, unit fields hold SI amounts) changes its unit within its quantity unchanged; plain numbers given a
/// unit are read as that unit, said loudly
fn conversion_factor(table: &str, name: &str, stored_type: &str, column_type: &str) -> Result<f64, String> {
	let column = format!("the column {table}.{name} holds {stored_type}, the class's field is {column_type}");
	let (stored_base, stored_unit) = split_unit(stored_type);
	let (base, unit) = split_unit(column_type);
	let widening = |sql_type: &str| LOSSLESS_ORDER.iter().position(|known| known.eq_ignore_ascii_case(sql_type));
	let lossless = matches!((widening(stored_base), widening(base)), (Some(from), Some(to)) if from <= to);
	let quantity = |unit: &str| crate::units::static_units::unit_type(unit).ok_or_else(|| format!("{column}: {unit} is no unit"));
	match (stored_unit, unit) {
		(Some(stored_unit), Some(unit)) => {
			let ((stored_quantity, _), (field_quantity, _)) = (quantity(stored_unit)?, quantity(unit)?);
			match stored_quantity == field_quantity {
				true => Ok(1.0),
				false => Err(format!("{column}: {stored_quantity} and {field_quantity} are different quantities, so it is not converted")),
			}
		}
		(None, Some(unit)) if NUMBER_COLUMNS.iter().any(|number| number.eq_ignore_ascii_case(stored_base)) => {
			let (_, per_unit) = quantity(unit)?;
			crate::diagnostic::report_runtime_warning(&format!("{column}: its plain numbers are read as {unit} (each × {per_unit} to the SI amount a unit field holds)"));
			Ok(per_unit)
		}
		(Some(_), None) => Err(format!("{column}: the unit would be lost, so it is not converted")),
		(None, None) if lossless && stored_base != base => Ok(1.0),
		_ => Err(format!("{column}: its values would lose data, so it is not converted")),
	}
}

/// `NUMERIC km` as NUMERIC and km
fn split_unit(column_type: &str) -> (&str, Option<&str>) {
	match column_type.split_once(' ') {
		Some((base, unit)) => (base, Some(unit)),
		None => (column_type, None),
	}
}

/// A field `[name, type, default]` as its column: name, SQL type, the SQL literal new rows of an added column take
fn column_of(field: &Node) -> Result<(String, String, String), String> {
	let parts = field.children();
	let ([name, field_type, default] | [name, field_type, default, _]) = parts.as_slice() else { return Err(format!("no field [name type default]: {}", field.serialize().trim())) };
	let field_type = field_type.drop_meta().name();
	let column_type = match field_type.as_str() {
		"int" | "i64" | "i32" | "bool" => "INTEGER".to_string(),
		"float" | "f64" | "f32" | "number" => "REAL".to_string(),
		"text" | "string" | "str" | "char" => "TEXT".to_string(),
		"" => String::new(),
		// a unit field (`distance: km`) holds SI amounts; its column keeps the unit for migrations
		unit if crate::units::static_units::unit_type(unit).is_some() => format!("{UNIT_COLUMN} {unit}"),
		other => return Err(format!("a field of type {other} is no column yet (notes/orm.md step 4: foreign keys)")),
	};
	let default = match (default.drop_meta(), split_unit(&column_type).0) {
		(Node::Empty, "INTEGER" | UNIT_COLUMN) => "0".to_string(),
		(Node::Empty, "REAL") => "0.0".to_string(),
		(Node::Empty, "TEXT") => "''".to_string(),
		(value, _) => literal(value)?,
	};
	Ok((name.drop_meta().name(), column_type, default))
}

fn literal(value: &Node) -> Result<String, String> {
	match value {
		Node::Empty => Ok("NULL".to_string()),
		Node::Number(Number::Int(number)) => Ok(number.to_string()),
		Node::Number(Number::Float(number)) => Ok(format!("{number:?}")),
		Node::True => Ok("1".to_string()),
		Node::False => Ok("0".to_string()),
		Node::Text(text) => Ok(format!("'{}'", text.replace('\'', "''"))),
		Node::Char(character) => literal(&Node::Text(character.to_string())),
		other => Err(format!("{} is no column value", other.serialize().trim())),
	}
}

/// "name" as an SQL identifier
fn quote(name: &str) -> String {
	format!("\"{}\"", name.replace('"', "\"\""))
}

fn list(items: Vec<Node>) -> Node {
	Node::List(items, Bracket::Square, Separator::Space)
}

/// The connection to the file's database, opened once per thread
fn connection(file: &str) -> Result<Handle, String> {
	let path = match file {
		crate::database_tables::TABLES_FILE => IN_MEMORY.to_string(),
		file => test_thread_copy(file).unwrap_or_else(|| file.to_string()),
	};
	if let Some(handle) = CONNECTIONS.with(|connections| connections.borrow().get(file).copied()) {
		return Ok(handle as Handle);
	}
	let sqlite = sqlite()?;
	let c_path = CString::new(path).map_err(|problem| problem.to_string())?;
	let mut handle: Handle = std::ptr::null_mut();
	if unsafe { (sqlite.open)(c_path.as_ptr(), &mut handle) } != SQLITE_OK {
		return Err(format!("{file}: {}", message(sqlite, handle)));
	}
	unsafe { (sqlite.busy_timeout)(handle, BUSY_MILLISECONDS) };
	CONNECTIONS.with(|connections| connections.borrow_mut().insert(file.to_string(), handle as usize));
	Ok(handle)
}

/// Under cargo's tests a named thread other than main (libtest names each test's thread) gets its own copy of a
/// sample's file, so tests running one sample at once neither lock nor change each other's rows
fn test_thread_copy(file: &str) -> Option<String> {
	let file = std::path::Path::new(file);
	let in_samples = file.parent().and_then(|folder| folder.file_name()).is_some_and(|folder| folder == SAMPLES_FOLDER);
	let folder = std::env::var(TEST_TABLES).ok().filter(|_| in_samples)?;
	let thread = std::thread::current().name().filter(|name| *name != "main")?.replace("::", ".");
	let folder = std::path::Path::new(&folder).join(thread);
	std::fs::create_dir_all(&folder).ok()?;
	Some(folder.join(file.file_name()?).to_string_lossy().into_owned())
}

fn message(sqlite: &Sqlite, database: Handle) -> String {
	unsafe { CStr::from_ptr((sqlite.errmsg)(database)) }.to_string_lossy().into_owned()
}

/// The rows the statement gives, its parameters bound in order
fn rows(database: Handle, sql: &str, parameters: &[Node]) -> Result<Vec<Vec<Node>>, String> {
	let sqlite = sqlite()?;
	let failure = |what: &str| format!("{what}: {} ({sql})", message(sqlite, database));
	let c_sql = CString::new(sql).map_err(|problem| problem.to_string())?;
	let mut statement: Handle = std::ptr::null_mut();
	if unsafe { (sqlite.prepare)(database, c_sql.as_ptr(), -1, &mut statement, std::ptr::null_mut()) } != SQLITE_OK {
		return Err(failure("sql"));
	}
	let result = (|| {
		for (index, parameter) in parameters.iter().enumerate() {
			bind(sqlite, statement, index as c_int + 1, parameter.drop_meta())?;
		}
		let mut found = vec![];
		loop {
			match unsafe { (sqlite.step)(statement) } {
				SQLITE_ROW => found.push(row(sqlite, statement)),
				SQLITE_DONE => return Ok(found),
				_ => return Err(failure("step")),
			}
		}
	})();
	unsafe { (sqlite.finalize)(statement) };
	result
}

fn bind(sqlite: &Sqlite, statement: Handle, index: c_int, value: &Node) -> Result<(), String> {
	let code = unsafe {
		match value {
			Node::Empty => (sqlite.bind_null)(statement, index),
			Node::Number(Number::Int(number)) => (sqlite.bind_int64)(statement, index, *number),
			Node::Number(Number::Float(number)) => (sqlite.bind_double)(statement, index, *number),
			Node::True => (sqlite.bind_int64)(statement, index, 1),
			Node::False => (sqlite.bind_int64)(statement, index, 0),
			Node::Text(_) | Node::Char(_) => {
				let text = CString::new(value.name()).map_err(|problem| problem.to_string())?;
				(sqlite.bind_text)(statement, index, text.as_ptr(), -1, SQLITE_TRANSIENT)
			}
			other => return Err(format!("{} is no column value", other.serialize().trim())),
		}
	};
	match code {
		SQLITE_OK => Ok(()),
		code => Err(format!("binding {} failed ({code})", value.serialize().trim())),
	}
}

fn row(sqlite: &Sqlite, statement: Handle) -> Vec<Node> {
	(0..unsafe { (sqlite.column_count)(statement) }).map(|column| unsafe {
		match (sqlite.column_type)(statement, column) {
			SQLITE_INTEGER => Node::int((sqlite.column_int64)(statement, column)),
			SQLITE_FLOAT => Node::float((sqlite.column_double)(statement, column)),
			SQLITE_TEXT => Node::Text(CStr::from_ptr((sqlite.column_text)(statement, column)).to_string_lossy().into_owned()),
			_ => Node::Empty,
		}
	}).collect()
}
