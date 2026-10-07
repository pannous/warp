// card web-apis (storage): `storage` is the program's store as a map keyed at run time: `storage["theme"] = "dark"`
// keeps a value, `storage["theme"]` reads it (ø when absent), `delete storage["theme"]` drops it, `keys(storage)` names
// them all; `storage.theme` is the same entry. Same store as `stored x = v` (lowering/stored_values.rs): stored.json
// natively, localStorage in the browser; code without a file keeps its values while the process runs (per thread)
use crate::is;
use warp::wasm_emitter::eval;

#[test]
fn a_value_kept_under_a_key_is_read_back() {
	is!("storage[\"theme_read\"] = \"dark\"; storage[\"theme_read\"]", "dark");
	is!("key = \"count_read\"; storage[key] = 41; storage[key] + 1", 42);
	is!("storage.color_read = \"red\"; storage.color_read", "red");
}

#[test]
fn an_absent_key_is_empty() {
	is!("storage[\"never_kept\"]", warp::node::Node::Empty);
}

#[test]
fn kept_values_last_into_the_next_run() {
	eval("storage[\"visits_next\"] = 3");
	is!("storage[\"visits_next\"]", 3);
}

#[test]
fn delete_drops_a_key() {
	is!("storage[\"gone_soon\"] = 1; delete storage[\"gone_soon\"]; storage[\"gone_soon\"]", warp::node::Node::Empty);
}

#[test]
fn keys_name_the_kept_values() {
	is!("storage[\"listed_key\"] = 1; contains(keys(storage), \"listed_key\")", true);
}

#[test]
fn a_stored_variable_and_its_key_are_one_entry() {
	is!("stored shared_entry = \"dark\"; shared_entry = \"light\"; storage[\"shared_entry\"]", "light");
}

// a soft keyword: a program that names its own storage keeps it
#[test]
fn a_program_variable_named_storage_stays_its_own() {
	is!("storage = {size: 1}; storage.size", 1);
}

// P188 (browser API names, warp-03's default): `local[k]` is localStorage, the same store as `storage[k]` (its alias)
// and `stored x`; `session[k]` is sessionStorage, a store of its own that lasts while the page's tab (natively: the
// process) does
#[test]
fn local_is_the_store_of_storage() {
	is!("local[\"theme_local\"] = \"dark\"; storage[\"theme_local\"]", "dark");
	is!("local.color_local = \"red\"; local[\"color_local\"]", "red");
	is!("local[\"gone_local\"] = 1; delete local[\"gone_local\"]; local[\"gone_local\"]", warp::node::Node::Empty);
	is!("local[\"listed_local\"] = 1; contains(keys(local), \"listed_local\")", true);
}

#[test]
fn session_is_a_store_of_its_own() {
	is!("session[\"tab_only\"] = \"open\"; session[\"tab_only\"]", "open");
	is!("session[\"apart\"] = 1; local[\"apart\"] = 2; session[\"apart\"] * 10 + local[\"apart\"]", 12);
	is!("session[\"session_listed\"] = 1; contains(keys(session), \"session_listed\")", true);
	is!("session[\"session_only\"] = 1; contains(keys(local), \"session_only\")", false);
}

#[test]
fn a_program_variable_named_local_or_session_stays_its_own() {
	is!("local = {size: 1}; local.size", 1);
	is!("session = [7 8]; session[1]", 8);
}
