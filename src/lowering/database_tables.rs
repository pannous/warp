//! Plain classes as database tables (card orm, notes/orm.md step 1): `people: [Person] = database.people` registers the
//! table people for the class Person. The list starts as the table's rows (`std_io("table", "open", …)` creates the
//! table or migrates it to the class's fields), `people.add(p)` inserts p and gives it its row's id, and a field change
//! `p.age += 1` of an instance with a row is written through (`std_io("table", "update", …)`). Natively the tables live
//! in `<program>.database.sqlite` (in memory for code without a file, database.rs); the browser has no tables yet.

use crate::node::{Bracket, Node, Separator};
use crate::operators::Op;
use crate::warp_parser::parse;
use std::collections::HashMap;

const DATABASE_WORDS: [&str; 2] = ["database", "indexedDB"];
/// `app.warp` keeps its tables in `app.database.sqlite`; inline code's tables are this name alone (in memory)
pub const TABLES_FILE: &str = "database.sqlite";
/// The row id every registered class gets, unless it declares one
const ID_FIELD: &str = "id";
const ADD_WORD: &str = "add";
/// the generated code's variables, written as these placeholders (`·` would parse as a product)
const ROW: &str = "table_row";
const ADDED: &str = "table_added";
const GENERATED_NAMES: [(&str, &str); 2] = [(ROW, "table·row"), (ADDED, "table·added")];
const SCHEMA_PLACEHOLDER: &str = "table_schema";
const VALUE_PLACEHOLDER: &str = "table_value";

/// A registered table: the list variable holding it and its class's fields (name, type, default)
struct Table {
	name: String,
	class: String,
	fields: Vec<(String, String, Option<Node>)>,
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
	let classes = class_bodies(&program);
	let mut tables: HashMap<String, Table> = HashMap::new();
	program.visit(&mut |part| if let Some((variable, table)) = registration(part, &classes) {
		tables.insert(variable, table);
	});
	if tables.is_empty() {
		return program;
	}
	let file = crate::modules::program_file().map_or_else(|| TABLES_FILE.to_string(), |file| file.with_extension(TABLES_FILE).to_string_lossy().into_owned());
	let registered: Vec<&str> = tables.values().map(|table| table.class.as_str()).collect();
	with_tables(with_row_ids(program, &registered), &tables, &file)
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
fn class_bodies(program: &Node) -> HashMap<String, Node> {
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
	Some((variable.drop_meta().name(), Table { name, class, fields }))
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

/// The registrations, inserts and field changes of the tables as their table calls, in every block
fn with_tables(node: Node, tables: &HashMap<String, Table>, file: &str) -> Node {
	match node {
		Node::List(statements, bracket, separator @ (Separator::Semicolon | Separator::Newline)) => {
			let statements = statements.into_iter().flat_map(|statement| table_statements(statement, tables, file)).collect();
			Node::List(statements, bracket, separator)
		}
		other => other.map_children(|child| with_tables(child, tables, file)),
	}
}

fn table_statements(statement: Node, tables: &HashMap<String, Table>, file: &str) -> Vec<Node> {
	if let Some(lowered) = opened(&statement, tables, file).or_else(|| inserted(&statement, tables, file)) {
		return lowered;
	}
	let updates = written_through(&statement, tables, file);
	[vec![with_tables(statement, tables, file)], updates].concat()
}

/// `people: [Person] = database.people` as the list of the table's rows, each an instance with its id
fn opened(statement: &Node, tables: &HashMap<String, Table>, file: &str) -> Option<Vec<Node>> {
	let Node::Key(target, Op::Assign, source) = statement.drop_meta() else { return None };
	let Node::Key(variable, Op::Colon, _) = target.drop_meta() else { return None };
	let table = tables.get(&variable.drop_meta().name())?;
	database_table(source)?;
	let columns = columns_of(table);
	// the row is [id, column values…] in the order of the columns
	let arguments: Vec<String> = table.fields.iter().map(|(name, _, _)| match name == ID_FIELD {
		true => format!("{ROW}#1"),
		false => format!("{ROW}#{}", 2 + columns.iter().position(|column| column == name).unwrap_or_default()),
	}).chain(implicit_id(table).then(|| format!("{ROW}#1"))).collect();
	let schema = Node::List(table.fields.iter().filter(|(name, _, _)| name != ID_FIELD).map(|(name, field_type, default)| {
		Node::List(vec![Node::Text(name.clone()), Node::Text(field_type.clone()), default.clone().unwrap_or(Node::Empty)], Bracket::Square, Separator::Space)
	}).collect(), Bracket::Square, Separator::Space);
	let code = format!("[{class}({arguments}) for {ROW} in std_io(\"table\", \"open\", [{name:?}, {SCHEMA_PLACEHOLDER}, {file:?}])]",
		class = table.class, arguments = arguments.join(", "), name = table.name);
	let rows = generated(&code, SCHEMA_PLACEHOLDER, schema);
	Some(vec![Node::Key(target.clone(), Op::Assign, Box::new(rows))])
}

/// The code with the placeholder as the node and the generated variables named
fn generated(code: &str, placeholder: &str, node: Node) -> Node {
	let names = GENERATED_NAMES.iter().map(|(written, name)| (written.to_string(), Node::Symbol(name.to_string())));
	crate::law::substitute(&parse(code), &names.chain([(placeholder.to_string(), node)]).collect())
}

/// The table's columns besides id, in field order
fn columns_of(table: &Table) -> Vec<String> {
	table.fields.iter().map(|(name, _, _)| name.clone()).filter(|name| name != ID_FIELD).collect()
}

fn implicit_id(table: &Table) -> bool {
	!table.fields.iter().any(|(name, _, _)| name == ID_FIELD)
}

/// `people.add(p)`: added to the list and inserted as a row, whose id p takes
fn inserted(statement: &Node, tables: &HashMap<String, Table>, file: &str) -> Option<Vec<Node>> {
	let Node::Key(list, Op::Dot, call) = statement.drop_meta() else { return None };
	let table = tables.get(&list.drop_meta().name())?;
	let Node::List(parts, _, _) = call.drop_meta() else { return None };
	let [word, value] = parts.as_slice() else { return None };
	if word.drop_meta().name() != ADD_WORD {
		return None;
	}
	let columns = columns_of(table);
	let names: Vec<String> = columns.iter().map(|column| format!("{column:?}")).collect();
	let values: Vec<String> = columns.iter().map(|column| format!("{ADDED}.{column}")).collect();
	let code = format!("{ADDED} = {VALUE_PLACEHOLDER}\n{list}.add({ADDED})\n{ADDED}.{ID_FIELD} = std_io(\"table\", \"insert\", [{table:?}, [{names}], [{values}], {file:?}])",
		list = list.drop_meta().name(), table = table.name, names = names.join(" "), values = values.join(" "));
	let lowered = generated(&code, VALUE_PLACEHOLDER, value.clone());
	Some(lowered.children())
}

/// After `p.age += 1` (any assignment of a field of a registered class): the change written to p's row, when p has one
fn written_through(statement: &Node, tables: &HashMap<String, Table>, file: &str) -> Vec<Node> {
	let Node::Key(target, op, _) = statement.drop_meta() else { return vec![] };
	if *op != Op::Assign && !op.is_compound_assign() {
		return vec![];
	}
	let Node::Key(instance, Op::Dot, field) = target.drop_meta() else { return vec![] };
	let (Node::Symbol(instance), field) = (instance.drop_meta(), field.drop_meta().name()) else { return vec![] };
	tables.values().filter(|table| field != ID_FIELD && columns_of(table).contains(&field)).map(|table| parse(&format!(
		"if {instance} is {class} and {instance}.{ID_FIELD} > 0 {{ std_io(\"table\", \"update\", [{name:?}, {instance}.{ID_FIELD}, {field:?}, {instance}.{field}, {file:?}]) }}",
		class = table.class, name = table.name))).collect()
}
