//! A parameter every call passes a map is looked up by key, also for a one-entry map (cards over-keys, inside-loop)
use crate::is;

#[test]
fn keys_of_a_one_entry_map_parameter() {
	is!("g(m) := { ks = keys(m); out = 0; for k in ks { out = m[k] }; out }; g({x: 2})", 2);
	is!("g(b) := { ks = keys(b); out = 0; for k in ks { out = b[k] }; out }; g({b: 2})", 2);
	is!("g(m) := { n = 0; for k in keys(m) { n += 1 }; n }; g({b: 2})", 1);
}
