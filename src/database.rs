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
const IN_MEMORY: &str = ":memory:";
const ID_COLUMN: &str = "id";

type Handle = *mut c_void;

/// The functions of the SQLite C API the tables use
struct Sqlite {
	open: unsafe extern "C" fn(*const c_char, *mut Handle) -> c_int,
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
	})
}

thread_local! {
	/// The open connections by file, kept while the thread runs (inline code's tables live as long as its connection)
	static CONNECTIONS: std::cell::RefCell<HashMap<String, usize>> = Default::default();
}

/// `std_io("table", member, arguments)`: open (create or migrate, give the rows), insert (give the id), update
pub fn call(member: &str, arguments: &[Node]) -> Result<Node, String> {
	let text = |node: &Node| match node.drop_meta() {
		Node::Text(text) => Ok(text.clone()),
		other => Err(format!("needs a text, got {}", other.serialize().trim())),
	};
	match (member, arguments) {
		("open", [table, schema, file]) => opened(&text(table)?, &schema.children(), &text(file)?),
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

/// The table's rows `[id, columns…]`, the table first created or migrated to the class's fields `[name, type, default]`
fn opened(table: &str, schema: &[Node], file: &str) -> Result<Node, String> {
	let database = connection(file)?;
	let columns = schema.iter().map(column_of).collect::<Result<Vec<_>, _>>()?;
	let definitions: Vec<String> = columns.iter().map(|(name, column_type, _)| format!("{} {column_type}", quote(name))).collect();
	let quoted_table = quote(table);
	rows(database, &format!("CREATE TABLE IF NOT EXISTS {quoted_table} ({ID_COLUMN} INTEGER PRIMARY KEY{})", definitions.iter().map(|definition| format!(", {definition}")).collect::<String>()), &[])?;
	let stored: Vec<(String, String)> = rows(database, &format!("PRAGMA table_info({quoted_table})"), &[])?.into_iter()
		.map(|row| (row[1].drop_meta().name(), row[2].drop_meta().name())).collect();
	for (name, column_type, default) in &columns {
		match stored.iter().find(|(stored_name, _)| stored_name == name) {
			None => { rows(database, &format!("ALTER TABLE {quoted_table} ADD COLUMN {} {column_type} DEFAULT {default}", quote(name)), &[])?; }
			Some((_, stored_type)) if !stored_type.eq_ignore_ascii_case(column_type) => {
				return Err(format!("the column {table}.{name} holds {stored_type}, the class's field is {column_type}: converting a column is not done yet (notes/orm.md step 7)"));
			}
			Some(_) => {}
		}
	}
	for (name, _) in stored.iter().filter(|(name, _)| name != ID_COLUMN && !columns.iter().any(|(column, _, _)| column == name)) {
		crate::diagnostic::report_runtime_warning(&format!("the table {table} keeps its column {name}, which the class no longer has (its data is kept)"));
	}
	let selected: Vec<String> = std::iter::once(ID_COLUMN.to_string()).chain(columns.iter().map(|(name, _, _)| quote(name))).collect();
	let rows = rows(database, &format!("SELECT {} FROM {quoted_table} ORDER BY {ID_COLUMN}", selected.join(", ")), &[])?;
	Ok(list(rows.into_iter().map(list).collect()))
}

/// A field `[name, type, default]` as its column: name, SQL type, the SQL literal new rows of an added column take
fn column_of(field: &Node) -> Result<(String, String, String), String> {
	let parts = field.children();
	let [name, field_type, default] = parts.as_slice() else { return Err(format!("no field [name type default]: {}", field.serialize().trim())) };
	let field_type = field_type.drop_meta().name();
	let column_type = match field_type.as_str() {
		"int" | "i64" | "i32" | "bool" => "INTEGER",
		"float" | "f64" | "f32" | "number" => "REAL",
		"text" | "string" | "str" | "char" => "TEXT",
		"" => "",
		other => return Err(format!("a field of type {other} is no column yet (notes/orm.md step 4: foreign keys)")),
	};
	let default = match (default.drop_meta(), column_type) {
		(Node::Empty, "INTEGER") => "0".to_string(),
		(Node::Empty, "REAL") => "0.0".to_string(),
		(Node::Empty, "TEXT") => "''".to_string(),
		(value, _) => literal(value)?,
	};
	Ok((name.drop_meta().name(), column_type.to_string(), default))
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
		crate::database_tables::TABLES_FILE => IN_MEMORY,
		file => file,
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
	CONNECTIONS.with(|connections| connections.borrow_mut().insert(file.to_string(), handle as usize));
	Ok(handle)
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
