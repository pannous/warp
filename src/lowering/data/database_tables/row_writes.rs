//! Row writes: `people.add(p)` inserts, a changed field updates its column, `save p` runs in a transaction
use super::*;

/// `people.add(p)`: added to the list and inserted as a row, whose id p takes; a foreign key stores the id of its row
/// (an instance without one is an error)
pub(super) fn inserted(statement: &Node, tables: &BTreeMap<String, Table>, file: &str) -> Option<Vec<Node>> {
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
pub(super) fn table_mutation<'a>(
	statement: &'a Node,
	tables: &'a BTreeMap<String, Table>,
) -> Option<(String, &'a Table, String, &'a Node)> {
	let Node::Key(list, Op::Dot, call) = statement.drop_meta() else {
		return None;
	};
	let list = list.drop_meta().name();
	let table = tables.get(&list)?;
	let Node::List(parts, _, _) = call.drop_meta() else {
		return None;
	};
	let [word, value] = parts.as_slice() else { return None };
	Some((list, table, word.drop_meta().name(), value))
}

/// The INSERT of ADDED, which takes the row's id, and `people.add(ADDED)`; a foreign key without its row is an error
pub(super) fn insert_code(table: &Table, list: &str, file: &str) -> String {
	let columns = columns_of(table);
	let names: Vec<String> = columns.iter().map(|column| format!("{column:?}")).collect();
	let values: Vec<String> = columns.iter().map(|column| column_value(table, ADDED, column)).collect();
	let checks = table.references.iter().map(|(field, target)| format!(
		"if {ADDED}.{field} != ø and {ADDED}.{field}.{ID_FIELD} == 0 {{ raise \"{class}.{field} is no row of {target}: add it to its table first\" }}\n", class = table.class));
	// the id first: the identity map keeps the instance by it
	format!("{checks}{ADDED}.{ID_FIELD} = std_io(\"table\", \"insert\", [{table:?}, [{names}], [{values}], {file:?}])\n{list}.add({ADDED})",
		checks = checks.collect::<String>(), table = table.name, names = names.join(" "), values = values.join(" "))
}

/// `red.players.add(p)` of a one-to-many field, anywhere (`d.team.players.add(d)` too): p's key points to red, written
/// to p's row, or p inserted into people when it has none (card orm-nested); the value p
pub(super) fn added_to_members(node: &Node, tables: &BTreeMap<String, Table>, file: &str) -> Option<Node> {
	let Node::Key(members_of, Op::Dot, call) = node.drop_meta() else {
		return None;
	};
	let Node::Key(owner, Op::Dot, field) = members_of.drop_meta() else {
		return None;
	};
	let Node::List(parts, _, _) = call.drop_meta() else {
		return None;
	};
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
pub(super) fn column_value(table: &Table, instance: &str, column: &str) -> String {
	match table.reference(column) {
		Some(_) => key_id(instance, column, table.is_optional(column)),
		None => format!("{instance}.{column}"),
	}
}

/// After `p.age += 1` (any assignment of a field of a registered class): the change written to p's row, when p has one
pub(super) fn written_through(statement: &Node, tables: &BTreeMap<String, Table>, file: &str) -> Vec<Node> {
	let Node::Key(target, op, _) = statement.drop_meta() else {
		return vec![];
	};
	if *op != Op::Assign && !op.is_compound_assign() {
		return vec![];
	}
	let Node::Key(instance, Op::Dot, field) = target.drop_meta() else {
		return vec![];
	};
	let (Node::Symbol(instance), field) = (instance.drop_meta(), field.drop_meta().name()) else {
		return vec![];
	};
	// the instance as a placeholder: a generated one (`people·element`) would parse as a product
	tables
		.values()
		.filter(|table| field != ID_FIELD && columns_of(table).contains(&field))
		.map(|table| {
			generated(
				&format!(
					"if {INSTANCE_PLACEHOLDER} is {class} and {INSTANCE_PLACEHOLDER}.{ID_FIELD} > 0 {{ {update} }}",
					class = table.class,
					update = column_update(table, INSTANCE_PLACEHOLDER, &field, file)
				),
				[(INSTANCE_PLACEHOLDER, Node::Symbol(instance.clone()))],
			)
		})
		.collect()
}

/// The UPDATE of one column of `instance`'s row
pub(super) fn column_update(table: &Table, instance: &str, column: &str, file: &str) -> String {
	format!(
		"std_io(\"table\", \"update\", [{name:?}, {instance}.{ID_FIELD}, {column:?}, {value}, {file:?}])",
		name = table.name,
		value = column_value(table, instance, column)
	)
}

/// `save p`, anywhere (`print(save p)` of a standalone build too): every column of p's row written, the value p; an
/// instance of a table's class without a row is an error
/// `transaction { block }` as the block between BEGIN and COMMIT, rolled back and raised again when it fails; its value
/// the block's
pub(super) fn in_transaction(node: &Node, file: &str, open: &[String]) -> Option<Node> {
	let Node::List(parts, _, Separator::Space) = node.drop_meta() else {
		return None;
	};
	let [word, block] = parts.as_slice() else { return None };
	if word.drop_meta().name() != TRANSACTION_WORD || !block.is_block() {
		return None;
	}
	let code = format!("std_io(\"table\", \"begin\", [{file:?}])\n\
		{TRANSACTION_VALUE} = try {VALUE_PLACEHOLDER} catch {FAILURE} {{ std_io(\"table\", \"rollback\", [{file:?}]); {restores}raise {FAILURE} }}\n\
		std_io(\"table\", \"commit\", [{file:?}])\n{TRANSACTION_VALUE}",
		restores = (0..open.len()).map(|index| format!("{RESTORE}_{index}(); ")).collect::<String>());
	// each open table's restore, `people·restore` (written as a placeholder: `·` would parse as a product)
	let restores: Vec<(String, Node)> = open
		.iter()
		.enumerate()
		.map(|(index, variable)| (format!("{RESTORE}_{index}"), Node::Symbol(lazy_name(variable, "restore"))))
		.collect();
	let placeholders = restores
		.iter()
		.map(|(placeholder, name)| (placeholder.as_str(), name.clone()))
		.chain([(VALUE_PLACEHOLDER, block.clone())]);
	Some(Node::List(generated(&code, placeholders).children(), Bracket::Round, Separator::Semicolon))
}

pub(super) fn saved(node: &Node, tables: &BTreeMap<String, Table>, file: &str) -> Option<Node> {
	let Node::List(parts, _, Separator::Space) = node.drop_meta() else {
		return None;
	};
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
	Some(Node::List(
		generated(&code, [(VALUE_PLACEHOLDER, value.clone())]).children(),
		Bracket::Round,
		Separator::Semicolon,
	))
}
