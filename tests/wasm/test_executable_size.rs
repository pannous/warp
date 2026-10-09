// Card g_gFs8 (user: "quicksort: enormous file size of simple algorithms should be reduced with tree shaking"): an
// executable carries only the functions its program reaches from what the stub calls (notes/aot.md)
use std::path::{Path, PathBuf};
use warp_runtime::standalone::{embedded_machine_code, standalone_engine, without_machine_code};

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");
/// samples/quicksort.warp carried 198768 bytes of machine code before tree shaking and deflating (~45 KB after)
const QUICKSORT_CARRIED_LIMIT: usize = 60_000;

/// `warp build --exe` of `source` as <name>.warp; the executable's path
fn built_executable(name: &str, source: &str) -> PathBuf {
	let source_path = PathBuf::from(OUTPUT_DIRECTORY).join(format!("{name}.warp"));
	std::fs::write(&source_path, source).unwrap();
	let build = crate::common::warp_command().env("WARP_RUNTIME_STUB", crate::common::runtime_stub()).args(["build", "--exe"]).arg(&source_path).output().unwrap();
	assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
	source_path.with_extension(std::env::consts::EXE_EXTENSION)
}

fn carried_exports(executable: &Path) -> Vec<String> {
	let machine_code = embedded_machine_code(executable).expect("the executable carries its program");
	let engine = standalone_engine().unwrap();
	// SAFETY: warp's own output, built by this test
	let module = unsafe { wasmtime::Module::deserialize(&engine, &machine_code).unwrap() };
	module.exports().map(|export| export.name().to_string()).collect()
}

fn printed_by(executable: &Path) -> String {
	String::from_utf8_lossy(&std::process::Command::new(executable).output().unwrap().stdout).into_owned()
}

#[test]
fn test_executable_exports_only_what_the_stub_calls() {
	let executable = built_executable("size_square", "square(x) := x * x\nsquare(7)");
	assert_eq!(printed_by(&executable), "49\n");
	let exports = carried_exports(&executable);
	assert!(exports.contains(&"main".to_string()), "{exports:?}");
	assert!(!exports.iter().any(|name| name == "square" || name == "get_kind" || name == "list_at"), "{exports:?}");
}

#[test]
fn test_executable_keeps_its_handlers() {
	let executable = built_executable("size_on_exit", "on exit { print \"bye\" }\nprint \"hello\"");
	let printed = printed_by(&executable);
	assert!(printed.starts_with("hello\n") && printed.ends_with("bye\n"), "{printed}");
}

#[test]
fn test_quicksort_executable_is_tree_shaken() {
	let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/quicksort.warp")).unwrap();
	let executable = built_executable("size_quicksort", &source);
	let bytes = std::fs::read(&executable).unwrap();
	let carried = bytes.len() - without_machine_code(&bytes).len();
	assert!(carried < QUICKSORT_CARRIED_LIMIT, "{carried} bytes carried");
	assert!(printed_by(&executable).starts_with("Original: [64 34 25"));
}
