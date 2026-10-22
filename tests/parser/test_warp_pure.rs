//! `warp pure <file>`: lib/compiler/compiler.warp, the warp compiler as one warp program, compiles the program and its
//! module runs (card warp-pure, src/compiler/standalone.rs)
const HELLO: &str = "print(\"hello world\")\n";
/// Time for building compiler-self.wasm: ~60 s on the Mac, more on a cloud VM, past one turn's 60 s default
const SELF_BUILD_TIME_LIMIT_SECONDS: &str = "900";

/// `warp <command>` (pure or self), ready for a program; no run builds a missing compiler (card installed-compilers).
/// `warp self` gets its compiler built anew once per test run: a warm cloud VM keeps compiler-self.wasm of an earlier
/// job's lib/ in target/tmp (card cloud-lane-warm), and building it inside the first program's run took more than that
/// run's 60 s (card self-build-time-limit). compiler.wasm of `warp pure` is built only when missing next to the test's
/// binary: that takes ~540 s (warp_pure_runs_hello_world)
pub(crate) fn lane(command: &str) -> std::process::Command {
	static BUILT: [std::sync::OnceLock<()>; 2] = [std::sync::OnceLock::new(), std::sync::OnceLock::new()];
	let mut lane = crate::common::warp_command();
	let delivered = std::path::Path::new(lane.get_program()).with_file_name("compiler.wasm");
	if command == "self" || !delivered.exists() {
		BUILT[(command == "self") as usize].get_or_init(|| build_compiler(command));
	}
	lane.args(["--no-ask", command]).env("WARP_NO_WINDOW", "1").env("WARP_HINTS", "0");
	lane
}

fn build_compiler(command: &str) {
	let built = crate::common::warp_command()
		.args(["rebuild", command])
		.env("WARP_TIME_LIMIT", SELF_BUILD_TIME_LIMIT_SECONDS)
		.output()
		.expect("warp runs");
	assert!(built.status.success(), "warp rebuild {command}: {}", String::from_utf8_lossy(&built.stderr));
}

#[test]
#[ignore = "slow: compiles all of compiler.warp, 540 s of the 600 s test limit at opt-level 2 (cloud, 2026-10-21); run with --ignored"]
fn warp_pure_runs_hello_world() {
	let folder = crate::common::scratch_directory("warp_pure");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join("hello.warp");
	std::fs::write(&program, HELLO).unwrap();
	let run = lane("pure").arg(&program).output().expect("warp runs");
	let printed = String::from_utf8_lossy(&run.stdout);
	assert!(printed.lines().next() == Some("hello world"), "{printed}\n{}", String::from_utf8_lossy(&run.stderr));
}

/// Loops and text concatenation under `warp self`: analyzer.warp's loop kind of a body ending in a non-group (`{ 2 }`),
/// node_matches.warp's `True:` arm of node_text (cards match-bool-arm, self-compiled-parity)
#[test]
fn warp_self_runs_loops_and_texts() {
	let code = "n = 0\nwhile n < 3 { n += 1 }\ns = \"a\"\ns += \"b\"\nprint s + n";
	let printed = said_by("self", "loops_and_texts", code);
	assert!(printed.lines().any(|line| line == "ab3"), "{printed}");
}

/// A program outside the emitter's core fails under `warp self` with the refusal, not a bare trap (card
/// compiler-refusal-loud)
#[test]
fn warp_self_names_the_refusal() {
	let printed = said_by("self", "refused", "nosuchword 3\n");
	assert!(printed.contains("outside the core: "), "{printed}");
}

/// The largest int as a literal under `warp self`: math.warp's bit_and, which sleb takes its low bits with, adds no
/// two values beyond 64 bits (card self-text-as-list: "checked add")
#[test]
fn warp_self_writes_the_largest_int() {
	let printed = said_by("self", "largest_int", "int64 x = 9223372036854775807\nprint x\n");
	assert!(printed.lines().any(|line| line == "9223372036854775807"), "{printed}");
}

/// A parameter name given twice, `middle(_, value, _)`: each parameter has a local of its own under `warp self`
/// (card self-text-as-list: "out of bounds array access")
#[test]
fn warp_self_takes_repeated_parameter_names() {
	let printed = said_by("self", "repeated_parameters", "def middle(_, value, _) := value\nprint middle(1, 2, 3)\n");
	assert!(printed.lines().any(|line| line == "2"), "{printed}");
}

/// What `warp <command>` (pure or self) says of the program `code`: its output and errors
fn said_by(command: &str, name: &str, code: &str) -> String {
	let folder = crate::common::scratch_directory(&format!("warp_{command}"));
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join(format!("{name}.warp"));
	std::fs::write(&program, code).unwrap();
	let run = lane(command).arg(&program).output().expect("warp runs");
	String::from_utf8_lossy(&run.stdout).to_string() + &String::from_utf8_lossy(&run.stderr)
}

fn pure_said(name: &str, code: &str) -> String {
	said_by("pure", name, code)
}

/// A form beyond the emitter's core is the compile's error, "outside the core: …" (card pure-not-a-list: compiler.warp
/// wrote the refusal's text as the module's bytes and failed with "not a list")
#[test]
#[ignore = "slow: compiles all of compiler.warp, as warp_pure_runs_hello_world; run with --ignored"]
fn warp_pure_says_what_it_refuses() {
	let said = pure_said("refused", "nosuchword 3\n");
	assert!(said.contains("outside the core: the unknown name nosuchword"), "{said}");
}

/// A library word compiles through the pure lane (card pure-not-a-list)
#[test]
fn warp_pure_runs_a_library_word() {
	let said = pure_said("library_word", "print sum [1, 2]\n");
	assert!(said.lines().next() == Some("3"), "{said}");
}

/// `warp self <file>`: compiler.warp compiled by itself (the pure lane) compiles the program (card self-hello)
#[test]
#[ignore = "redundant: covered by parser::test_warp_pure::warp_self_runs_loops_and_texts"]
fn warp_self_runs_hello_world() {
	let folder = crate::common::scratch_directory("warp_self");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join("hello.warp");
	std::fs::write(&program, HELLO).unwrap();
	let run = lane("self").arg(&program).output().expect("warp runs");
	let printed = String::from_utf8_lossy(&run.stdout);
	assert!(printed.lines().next() == Some("hello world"), "{printed}\n{}", String::from_utf8_lossy(&run.stderr));
}

/// `warp pure <code>` and `warp self <code>` compile inline code as plain `warp <code>` runs it (card pure-inline: the
/// command word was read as a variable, "undefined variable: pure")
#[test]
fn warp_pure_and_self_run_inline_code() {
	for command in ["pure", "self"] {
		let run = lane(command).arg("x=6;x*7").output().expect("warp runs");
		let said = String::from_utf8_lossy(&run.stdout).to_string() + &String::from_utf8_lossy(&run.stderr);
		assert!(said.lines().any(|line| line.trim_start_matches("» ") == "42"), "{command}: {said}");
	}
}

/// Two used files declaring one function differently are an error naming both under `warp self`, as in the Rust lane
/// (card module-name-clash: uses.warp silently kept one, printing "aa")
#[test]
fn warp_self_refuses_a_function_declared_in_two_used_files() {
	let folder = crate::common::scratch_directory("warp_self");
	std::fs::create_dir_all(&folder).unwrap();
	for side in ["a", "b"] {
		let file = format!("named(x) := \"{side}\"\nuse_{side}(x) := named(x)\n");
		std::fs::write(folder.join(format!("clash_{side}.warp")), file).unwrap();
	}
	let code = "use \"clash_a.warp\"\nuse \"clash_b.warp\"\nprint(use_a(1) + use_b(1))\n";
	let printed = said_by("self", "clash", code);
	assert!(printed.contains("is declared in both") && printed.contains("clash_b.warp"), "{printed}");
}

/// What `warp pure` and `warp self` print for the program, each run's lines
fn lanes_print(name: &str, code: &str, expected: &[&str]) {
	for command in ["pure", "self"] {
		let said = said_by(command, name, code);
		let lines: Vec<&str> = said.lines().collect();
		assert!(lines.starts_with(expected), "{command}: {said}");
	}
}

/// sum is the prelude's fold, `sum(xs) := fold(xs, 0, +)`, a range's sum its closed form: three billion items in no
/// time (card sum-plain-fold)
#[test]
fn warp_pure_and_self_sum_as_a_fold() {
	let code = "print sum([1, 2, 3])\nprint sum(1 to 3000000000)\nprint sum(3 .. 7)\nprint sum([1, 2, 3] .* 2)\n";
	lanes_print("sum_fold", code, &["6", "4500000001500000000", "18", "12"]);
}

/// A program's own sum replaces the prelude's, its range rule too (cards sum-plain-fold, rust-lane-program-2)
#[test]
fn warp_pure_and_self_take_a_programs_own_sum() {
	lanes_print("own_sum", "sum(xs) := 42\nprint sum([1, 2, 3])\nprint sum(1 to 10)\n", &["42", "42"]);
}

/// An operator is a fold's function: `*`, and the words `or`, `and` (card sum-plain-fold)
#[test]
fn warp_pure_and_self_fold_with_operators() {
	let code = "print fold([2, 3, 4], 1, *)\nprint fold([no, yes], no, or)\nprint product([2, 3, 4])\n";
	lanes_print("operator_folds", code, &["24", "yes", "24"]);
}
