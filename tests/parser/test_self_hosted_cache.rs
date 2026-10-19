//! The self-hosted pipeline's stage program (parser.warp and the pass twins) compiles once and is kept in the module
//! cache: the next run of warp with WARP_SELF_HOSTED loads it instead of compiling it again (seconds), so it lowers only
//! its own program
const CACHE_FOLDER: &str = "self_hosted_stage_cache";
const CACHE_IN_A_FOLDER: &str = "self_hosted_stage_cache_in_a_folder";
/// A folder with text.warp and list.warp
const SHADOWING_FOLDER: &str = "probes/self_hosting/emitter/core";
const HOST_WORDS_PROGRAM: &str = "probes/self_hosting/emitter/host_words.warp";

fn self_hosted_run(cache: &std::path::Path, code: &str) -> (String, String) {
	self_hosted_run_in(".", cache, code)
}

fn self_hosted_run_in(folder: &str, cache: &std::path::Path, code: &str) -> (String, String) {
	run_switched("1", folder, cache, code)
}

fn run_switched(switch: &str, folder: &str, cache: &std::path::Path, code: &str) -> (String, String) {
	let run = crate::common::warp_command()
		.current_dir(folder)
		.env("WARP_SELF_HOSTED", switch)
		.env("WARP_MODULE_CACHE", cache)
		.env("WARP_PHASE_TIMES", "1")
		.arg(code)
		.output()
		.expect("warp runs");
	(String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned())
}

#[test]
fn the_stage_program_is_compiled_once() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	let (first, _) = self_hosted_run(&cache, "3 + 4");
	assert!(first.contains('7'), "{first}");
	let (again, phases) = self_hosted_run(&cache, "3 + 4");
	assert!(again.contains('7'), "{again}");
	assert!(phases.contains("lowering, all (1×)"), "only the program is lowered: {phases}");
}

/// emitter.warp emits a program inside its core, so the Rust emitter never runs; one beyond it falls back to Rust
#[test]
fn the_warp_emitter_emits_inside_its_core() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	self_hosted_run(&cache, "3 + 4"); // the stage programs compile with the Rust emitter, once
	let (inside, phases) = self_hosted_run(&cache, "square(x: int) := x * x; print(square(5)); square(3)");
	assert!(inside.contains("25") && inside.contains('9'), "{inside}");
	assert!(!phases.contains("emit ("), "the Rust emitter ran: {phases}");
	let (class, phases) = self_hosted_run(&cache, "class Point { x: int }; p = Point(3); p.x + 4");
	assert!(class.contains('7'), "{class}");
	assert!(!phases.contains("emit ("), "a class instance is inside the core: {phases}");
	let (closure, phases) = self_hosted_run(&cache, "x = 3; f = y => x + y; f(4)");
	assert!(closure.contains('7'), "{closure}");
	assert!(!phases.contains("emit ("), "a closure over a variable is inside the core: {phases}");
	let (beyond, phases) = self_hosted_run(&cache, "x = 3; g = y => z => x + y + z; g(1)(2)");
	assert!(beyond.contains('6'), "{beyond}");
	assert!(phases.contains("emit ("), "a closure giving a closure is beyond the core: {phases}");
}

/// The stage programs compile from warp's lib/ alone: a folder whose text.warp and list.warp would shadow the standard
/// modules (probes/self_hosting/emitter/core) changes nothing of them
#[test]
fn the_stage_programs_ignore_the_working_folder() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_IN_A_FOLDER);
	let _ = std::fs::remove_dir_all(&cache);
	let (value, phases) = self_hosted_run_in(SHADOWING_FOLDER, &cache, "area(side: int) := side * side; area(3) + 4");
	assert!(value.contains("13"), "{value} {phases}");
}

/// WARP_SELF_HOSTED=front: the warp front end with the Rust emitter, for a count of the front end alone
#[test]
fn the_front_end_alone_leaves_emitting_to_rust() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	self_hosted_run(&cache, "3 + 4"); // the stage programs compile with the Rust emitter, once
	let (value, phases) = run_switched("front", ".", &cache, "square(x: int) := x * x; square(3)");
	assert!(value.contains('9'), "{value}");
	assert!(phases.contains("emit ("), "the Rust emitter emits: {phases}");
}

/// emitter.warp imports the host words on numbers a program calls (clock, random_below) from the host module
#[test]
fn the_warp_emitter_imports_host_words() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	self_hosted_run(&cache, "3 + 4"); // the stage programs compile with the Rust emitter, once
	let program = std::fs::read_to_string(HOST_WORDS_PROGRAM).unwrap();
	let (value, phases) = self_hosted_run(&cache, &program);
	assert!(value.contains('5'), "{value}");
	assert!(!phases.contains("emit ("), "the Rust emitter ran: {phases}");
}

/// A module of emitter.warp fails with the runtime error Rust's names (index range, fractional index, zero divisor),
/// through runtime.warp words of the error's name and the exported trap_detail (src/wasm_emitter/control/traps.rs)
#[test]
#[ignore = "card node-conditions: xs#4 gives \"index out of range\" without Rust's \"4 not in 1…3\" (emit-closures/emitter-truthy batch)"]
fn the_warp_emitter_fails_with_rusts_runtime_errors() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	self_hosted_run(&cache, "3 + 4");
	let failures = [
		("xs = [1 2 3]; xs#4", "index out of range: 4 not in 1…3"),
		("xs = [1 2]; xs#0 = 5", "index out of range: 0 not in 1…2"),
		("xs = [3 1 2]; xs#(3/2)", "index must be an integer"),
		("1/0", "divide by zero"),
	];
	for (program, message) in failures {
		let (value, phases) = self_hosted_run(&cache, program);
		assert!(value.contains(message), "{program}: {value}");
		assert!(!phases.contains("emit ("), "{program}: the Rust emitter ran: {phases}");
	}
}
