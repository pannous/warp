//! Card runtime-speed: a map no hash-table variable holds (a parameter, a returned map) is found and filled in constant
//! time per entry: a lookup or field set that walked the same map last time builds an index of it, kept while only
//! field sets and appends change the map
use crate::is;

/// Entries filled and looked up, and the milliseconds that may take: well under, quadratic takes seconds
const ENTRIES: i64 = 20_000;
const MILLISECONDS: i64 = 1500;
const SUM: i64 = ENTRIES * (ENTRIES - 1) / 2;

fn timed(definitions: &str, work: &str) -> String {
	format!("n = {ENTRIES}; {definitions}; t0 = clock(); {work}; if clock() - t0 < {MILLISECONDS} then s else -1")
}

#[test]
fn a_map_parameter_fills_and_finds_in_linear_time() {
	let fill = "def fill(m, n) { for i in 0..n { m[\"k\" + i] = i } }";
	let look = "def look(m, n) { s = 0; for i in 0..n { s += m[\"k\" + i] }; s }";
	is!(&timed(&format!("{fill}; {look}"), "m = {}; fill(m, n); s = look(m, n)"), SUM);
	// a returned map, looked up where it lands
	let build = "def build(n) { m = {}; for i in 0..n { m[\"k\" + i] = i }; m }";
	is!(&timed(build, "m = build(n); s = 0; for i in 0..n { s += m[\"k\" + i] }"), SUM);
}

#[test]
fn an_indexed_map_changed_in_place_finds_the_change() {
	let fill = "def fill(m, n) { for i in 0..n { m[\"k\" + i] = i } }";
	// a field set again, an entry replaced by position, a field set and a field added after a lookup
	is!(&format!("{fill}; m = {{}}; fill(m, 40); m.k3 = 7; m[\"k3\"]"), 7);
	is!(&format!("{fill}; def at(m, k) {{ m[k] }}; m = {{}}; fill(m, 40); x = at(m, \"k5\") + at(m, \"k6\"); m#1 = (q: 9); y = try at(m, \"k0\"); [at(m, \"q\"), at(m, \"k9\"), y is error]"), warp::parse("[9 9 yes]"));
	is!(&format!("{fill}; def has(m, k) {{ m[k] }}; m = {{}}; fill(m, 40); x = has(m, \"k1\"); m.k1 = 5; m.z = 2; [has(m, \"k1\"), has(m, \"z\"), count(m)]"), warp::parse("[5 2 41]"));
}
