// card how-store: the store word alone is all its kept values as a map, so `values(storage)`, `entries(storage)` and
// `for key, value in storage` read them all; `keys(storage)` names them (test_storage.rs)
use crate::is;

#[test]
fn the_store_alone_is_its_values() {
	is!("session[\"all_a\"] = 1; session[\"all_b\"] = 2; s = session; s[\"all_a\"] + s[\"all_b\"]", 3);
	is!("database[\"all_db\"] = 5; contains(values(database), 5)", true);
	is!("local[\"all_local\"] = \"kept\"; contains(values(local), \"kept\")", true);
}

#[test]
fn values_of_a_store() {
	is!("session[\"sum_a\"] = 20; session[\"sum_b\"] = 22; sum(values(session))", 42);
}

// the session store lasts while the thread runs: this test's thread keeps one entry
#[test]
fn entries_of_a_store() {
	is!("use map; session[\"entry_only\"] = 7; count(entries(session))", 1);
}

#[test]
fn a_store_walked_key_by_key() {
	is!("session[\"walk_a\"] = 1; session[\"walk_b\"] = 2; total = 0; for key, value in session { total += value }; total", 3);
}

// card empty-store: a stored variable is in its store from its first run, not only after its first change
#[test]
fn a_stored_variable_is_among_the_store_values() {
	is!("stored shown_default default 41; storage[\"shown_default\"] + 1", 42);
	is!("stored listed_default default 1; contains(keys(storage), \"listed_default\")", true);
}
