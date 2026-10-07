// Cards over-keys and index-hint: `m[k]` with `k` from `keys(m)` looks a key up, also in a function given a one-entry
// map (it was taken for a positional index: the parameter became a list and `{b: 2}` its two items b and 2), and the
// indexing hint does not propose `m#(k+1)` for it
use crate::is;
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasp_parser::WaspParser;

#[test]
fn a_key_of_a_one_entry_map_parameter_looks_its_value_up() {
	is!("g(m) := { r = 0; for k in keys(m) { r = m[k] }; r }; g({b: 2})", 2);
	is!("g(m) := { r = 0; for k in m.keys { r = m[k] }; r }; g({b: 2})", 2);
	is!("g(m) := { n = 0; for k in keys(m) { n += 1 }; n }; g({b: 2})", 1);
	is!("g(m) := { r = 0; i = 0; while i < 1 { k = keys(m)#1; r = m[k]; i++ }; r }; g({b: 2})", 2);
	is!("merge(a, b) := { out = a; for k in keys(b) { out[k] = b[k] }; out }; string(merge({a: 1}, {b: 2}))", "{a:1 b:2}");
}

#[test]
fn a_key_variable_gets_no_indexing_hint() {
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| WaspParser::parse("merge(a, b) := { out = a; for k in keys(b) { out[k] = b[k] }; out }"));
	assert!(hints.iter().all(|hint| !hint.reason.contains("indexing")), "{hints:?}");
	let (_, hints) = capture_hints(|| WaspParser::parse("xs = [1 2]; for k in keys(m) { 0 }; xs[k]"));
	assert!(hints.iter().any(|hint| hint.reason.contains("indexing")), "outside the loop k is no key variable: {hints:?}");
}
