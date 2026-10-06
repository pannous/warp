//! The standard library module map (notes/stdlib.md)
use crate::is;
use warp::wasp_parser::parse;

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
