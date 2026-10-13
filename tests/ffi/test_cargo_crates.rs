//! `use cargo regex`: a crate from crates.io as a WebAssembly component (src/cargo_crates.rs): warp generates a wrapper
//! crate exporting a WIT world of the crate's public functions, builds it for wasm32-wasip2 once per machine
//! (~/.cache/warp/crates) and loads it like `use wasm` (src/components.rs). Real crates, built on first use.
use crate::common::fails_with;
use crate::is;

#[test]
fn test_cargo_crate_free_functions() {
	is!("use cargo strsim; strsim.levenshtein(\"kitten\", \"sitting\")", 3);
	is!("use crate \"strsim\"; strsim.hamming(\"hello\", \"hallo\")", 1);
	is!("use cargo strsim as s; s.jaro_winkler(\"abc\", \"abc\")", 1.0);
}

#[test]
fn test_cargo_crate_struct_is_a_resource() {
	is!("use cargo regex; regex.escape(\"a.b\")", "a\\.b");
	is!("use cargo regex; r = regex.Regex(\"a+\"); r.is_match(\"caaat\")", true);
	is!("use cargo regex; r = regex.Regex(\"a+\"); r.replace_all(\"caaat\", \"-\")", "c-t");
	is!("use cargo regex; r = regex.Regex(\"[0-9]+\"); r.as_str()", "[0-9]+");
}

#[test]
fn test_cargo_crate_errors_are_loud() {
	fails_with("use cargo regex; regex.Regex(\"a(\")", "unclosed group");
	fails_with("use cargo no_such_crate_xyz_warp; 1", "no_such_crate_xyz_warp");
}
