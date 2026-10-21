//! The self-hosted pipeline's stage program (parser.warp and the pass twins) compiles once and is kept in the module
//! cache: the next run of warp with WARP_SELF_HOSTED loads it instead of compiling it again (seconds), so it lowers only
//! its own program
const CACHE_FOLDER: &str = "self_hosted_stage_cache";
const CACHE_IN_A_FOLDER: &str = "self_hosted_stage_cache_in_a_folder";
const CACHE_OF_LIBRARY_COPIES: &str = "self_hosted_stage_cache_of_library_copies";
const LIBRARY_COPIES: [&str; 2] = ["self_hosted_library_copy_a", "self_hosted_library_copy_b"];
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

fn copy_folder(from: &std::path::Path, to: &std::path::Path) {
	std::fs::create_dir_all(to).unwrap();
	for entry in std::fs::read_dir(from).unwrap().flatten() {
		let target = to.join(entry.file_name());
		if entry.path().is_dir() {
			copy_folder(&entry.path(), &target);
		} else {
			std::fs::copy(entry.path(), target).unwrap();
		}
	}
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
	let (curried, phases) = self_hosted_run(&cache, "x = 3; g = y => z => x + y + z; g(1)(2)");
	assert!(curried.contains('6'), "{curried}");
	assert!(!phases.contains("emit ("), "a closure giving a closure is inside the core: {phases}");
	let (caught, phases) = self_hosted_run(&cache, "try { fail(\"no\") } catch e { 5 }");
	assert!(caught.contains('5'), "{caught}");
	assert!(!phases.contains("emit ("), "a try is inside the core: {phases}");
	let (beyond, phases) = self_hosted_run(&cache, "go { 1 }; 2");
	assert!(beyond.contains('2'), "{beyond}");
	assert!(phases.contains("emit ("), "a task is beyond the core: {phases}");
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

/// WARP_SELF_HOSTED=0 (or off, no, false, empty) is the lane switched off, as WARP_HINTS=0: the Rust emitter emits
/// a program emitter.warp would take (card switch-self)
#[test]
fn a_zero_switch_is_off() {
	let cache = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(CACHE_FOLDER);
	for off in ["0", "off", ""] {
		let (value, phases) = run_switched(off, ".", &cache, "square(x: int) := x * x; square(3)");
		assert!(value.contains('9'), "{value}");
		assert!(phases.contains("emit ("), "WARP_SELF_HOSTED={off:?} ran the self-hosted emitter: {phases}");
	}
}

/// The kept stage programs are keyed by lib/'s contents, not its place: a worktree with the same lib/ reuses them
/// instead of compiling every stage again (card stage-key-content)
#[test]
fn an_identical_library_elsewhere_reuses_the_kept_stages() {
	let scratch = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
	let cache = scratch.join(CACHE_OF_LIBRARY_COPIES);
	let _ = std::fs::remove_dir_all(&cache);
	let copies = LIBRARY_COPIES.map(|copy| scratch.join(copy));
	for copy in &copies {
		let _ = std::fs::remove_dir_all(copy);
		copy_folder(std::path::Path::new("lib"), copy);
	}
	let run_with = |library: &std::path::Path| {
		let run = crate::common::warp_command()
			.env("WARP_SELF_HOSTED", "1")
			.env("WARP_MODULE_CACHE", &cache)
			.env("WARP_PHASE_TIMES", "1")
			.env("WARP_LIB", library)
			.arg("3 + 4")
			.output()
			.expect("warp runs");
		(String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned())
	};
	let (first, _) = run_with(&copies[0]);
	assert!(first.contains('7'), "{first}");
	let (again, phases) = run_with(&copies[1]);
	assert!(again.contains('7'), "{again}");
	assert!(phases.contains("lowering, all (1×)"), "only the program is lowered: {phases}");
}
