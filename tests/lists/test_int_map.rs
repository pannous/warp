//! Card int-map: a map keyed by numbers (P34: a number subscript keys a variable that starts as `{}`) works with any
//! number expression, not only a written one, and stays a hash table when it is a global: set, found and tested in O(1)
use crate::is;

/// Entries set, and the milliseconds that may take: ~100 at constant time per entry, minutes when each copies the map
pub(crate) const ENTRIES: i64 = 10_000;
const MILLISECONDS: i64 = 3000;

pub(crate) fn timed(statements: &str, result: &str) -> String {
	format!("t0 = clock(); {statements}; t1 = clock(); if t1 - t0 < {MILLISECONDS} then {result} else -1")
}

#[test]
fn a_number_variable_keys_an_empty_map() {
	is!("m = {}; k = 4; m[k] = 8; m[4]", 8);
	is!("m = {}; k = 4; m[4] = 8; m[k]", 8);
	is!("m = {}; for i in 1 to 20 { m[i] = i * 2 }; m[13] + count(m)", 46);
	is!("m = {}; m[2 + 2] = 8; m[4]", 8);
}

#[test]
fn a_number_is_found_in_a_map() {
	is!("m = {}; m[4] = 8; if 4 in m then 1 else 0", 1);
	is!("m = {}; k = 4; m[k] = 8; if k in m then 1 else 0", 1);
	is!("m = {}; m[4] = 8; if 5 in m then 1 else 0", 0);
	is!("m = {}; m[4] = 8; if m has 4 then 1 else 0", 1);
}

#[test]
fn a_global_map_is_set_in_a_function() {
	is!("m = {}; f(k, v) := { global m; m[k] = v }; f(9, 1); f(10, 2); m[9] + m[10]", 3);
	is!("m = {}; f(k) := { global m; m[k] }; m[3] = 7; f(3)", 7);
	is!("m = {}; f(k) := { global m; if k in m then 1 else 0 }; m[3] = 7; f(3) + f(4)", 1);
	is!("m = {}; f() := { global m; m = {} }; m[3] = 7; f(); count(m)", 0);
}

#[test]
fn an_entry_is_removed_by_its_number() {
	is!("m = {}; for i in 1 to 100 { m[i] = i }; for i in 1 to 99 { m.remove(i) }; [count(m), m[100]]", warp::parse("[1 100]"));
	is!("m = {}; f(k) := { global m; m.remove(k) }; m[1] = 1; m[2] = 2; f(1); if 1 in m then 0 else m[2]", 2);
	is!("m = {}; m[1] = 5; m.remove(1)", 5);
}

#[test]
fn a_global_map_takes_constant_time_per_entry() {
	let filled = format!("m = {{}}; put(k, v) := {{ global m; m[k] = v }}; for i in 1 to {ENTRIES} {{ put(i, i * 2) }}");
	is!(&timed(&filled, "m[777]"), 1554);
	let found = format!("{filled}; got(k) := {{ global m; if k in m then m[k] else 0 }}; s = 0; for i in 1 to {ENTRIES} {{ s += got(i) }}");
	is!(&timed(&found, "s"), ENTRIES * (ENTRIES + 1));
}

/// Four times the rows may take up to this many times as long: linear is ~4, quadratic ~16. A ratio of two runs in one
/// program holds on a loaded machine, where a bound in milliseconds does not (card flaky-linear)
const LINEAR_RATIO: i64 = 8;
/// The milliseconds the small load counts as at least: a fast engine (the browser's) loads it in a few, where a clock tick
/// and a warm-up weigh more than the rows
const SMALL_LOAD_FLOOR: i64 = 100;

/// The identity map of a table (notes/orm.md) finds a loaded row's instance by its id: loading is linear
#[test]
fn ten_thousand_rows_load_in_linear_time() {
	let loaded = |table: &str, rows: i64| format!("{table}: [Person] = database.int_map_{table}\nfor i in 1 to {rows} {{ {table}.add(Person(\"p\", i)) }}\n{table}_sum = 0; for p in {table} {{ {table}_sum += p.age }}");
	let (small, large) = (loaded("small", ENTRIES / 4), loaded("large", ENTRIES));
	let program = format!("class Person{{name: text; age: int}}\nt0 = clock()\n{small}\nt1 = clock()\n{large}\nt2 = clock()\nif t2 - t1 <= {LINEAR_RATIO} * max(t1 - t0, {SMALL_LOAD_FLOOR}) then large_sum else [t1 - t0, t2 - t1]");
	is!(&program, ENTRIES * (ENTRIES + 1) / 2);
}
