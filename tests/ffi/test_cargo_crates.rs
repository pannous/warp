//! `use cargo regex`: a crate from crates.io as a WebAssembly component (src/modules/cargo_crates.rs): warp generates a wrapper
//! crate exporting a WIT world of the crate's public functions, builds it for wasm32-wasip2 once per machine
//! (~/.cache/warp/crates) and loads it like `use wasm` (src/modules/components.rs). Real crates, built on first use.
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

#[test]
fn test_cargo_crate_texts_pass_as_bytes() {
	is!("use cargo hex; hex.encode(\"hi\")", "6869");
	is!("use cargo crc32fast; crc32fast.hash(\"abc\")", 891568578);
}

#[test]
fn test_cargo_crate_type_functions_fields_and_display() {
	is!("use cargo semver; v = semver.Version.parse(\"1.2.3\"); v.minor + 1", 3);
	is!("use cargo semver; semver.Version.parse(\"1.2.3\").to_string()", "1.2.3");
	is!("use cargo semver; r = semver.VersionReq.parse(\">=1.2\"); r.matches(semver.Version.parse(\"1.3.0\"))", true);
}

#[test]
fn test_cargo_crate_without_callable_functions_is_refused() {
	fails_with("use cargo sha2; 1", "no public function warp can call");
}

/// card use-cargo-downloads: CI builds the components of common crates (.github/workflows/crates.yml) as release
/// assets; `use cargo X` downloads one, checked against its SHA-256, before building it locally
#[test]
fn test_cargo_crate_prebuilt_download() {
	let directory = std::path::PathBuf::from(format!("scratch/crate-downloads-{}", std::process::id()));
	let wasm = warp::modules::cargo_crates::download("strsim", &directory).unwrap();
	let program = format!("use wasm \"{}\" as strsim; strsim.levenshtein(\"kitten\", \"sitting\")", wasm.display());
	is!(&program, 3);
	std::fs::remove_dir_all(&directory).unwrap();
	assert!(warp::modules::cargo_crates::download("no_such_crate_xyz_warp", &directory)
		.unwrap_err()
		.contains("not hosted"));
}

/// card use-cargo-debug-names: crate components keep their name section, so a trap's backtrace names the Rust
/// functions (demangled) instead of `<wasm function 2106>` (regex-automata itself is inlined)
#[test]
fn test_cargo_crate_trap_names_rust_functions() {
	fails_with("use cargo regex; r = regex.Regex(\"a\"); r.shortest_match_at(\"abc\", 10)", "RegexResource");
}
