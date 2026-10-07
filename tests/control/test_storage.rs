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
