//! The standard library module map (notes/stdlib.md)
use crate::is;
use warp::warp_parser::parse;

#[test]
fn use_map_brings_merge_and_map_values() {
	is!("use map; merge({a: 1, b: 2}, {b: 3, c: 4})", parse("{a:1 b:3 c:4}"));
	is!("use map; map_values({a: 1, b: 2}, x => x * 10)", parse("{a:10 b:20}"));
}

#[test]
fn a_one_entry_map_merges_too() {
	is!("use map; merge({a: 1}, {b: 2})", parse("{a:1 b:2}"));
	is!("use map; map_values({a: 1}, x => x + 1)", parse("{a:2}"));
}

#[test]
fn use_map_inverts_picks_and_builds_from_pairs() {
	is!("use map; m = invert({a:\"x\" b:\"y\"}); m.get(\"y\")", "b");
	is!("use map; count(keys(pick({a:1 b:2 c:3}, [\"a\", \"c\", \"z\"])))", 2);
	is!("use map; m = from_pairs([[\"a\", 1], [\"b\", 2]]); m.get(\"b\")", 2);
}

#[test]
fn use_map_omits_and_filters_values() {
	is!("use map; m = omit({a:1 b:2 c:3}, [\"b\"]); count(keys(m)) * 10 + m.get(\"c\")", 23);
	is!("use map; m = filter_values({a:1 b:5 c:7}, v => v > 2); count(keys(m)) * 10 + m.get(\"b\")", 25);
}

#[test]
fn use_map_lists_its_entries() {
	is!("use map; entries({ab:1, cd:2})", parse(r#"[["ab" 1] ["cd" 2]]"#));
}

#[test]
fn use_map_gets_with_a_fallback() {
	is!("use map; [get_or({ab:1}, \"cd\", 0), get_or({ab:1}, \"ab\", 0)]", warp::ints(vec![0, 1]));
}

#[test]
fn use_map_finds_a_key_by_its_value() {
	is!("use map; find_key({ab:1, cd:2}, v => v == 2)", "cd");
}
