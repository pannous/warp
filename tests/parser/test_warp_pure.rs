//! `warp pure <file>`: lib/compiler/compiler.warp, the warp compiler as one warp program, compiles the program and its
//! module runs (card warp-pure, src/compiler/standalone.rs)
const HELLO: &str = "print(\"hello world\")\n";
/// Time for building compiler-self.wasm: ~60 s on the Mac, more on a cloud VM, past one turn's 60 s default
const SELF_BUILD_TIME_LIMIT_SECONDS: &str = "900";

/// `warp <command>` (pure or self), ready for a program. `warp self` gets its compiler built anew once per test run:
/// a warm cloud VM keeps compiler-self.wasm of an earlier job's lib/ in target/tmp (card cloud-lane-warm), and building
/// it inside the first program's run took more than that run's 60 s (card self-build-time-limit). compiler.wasm of
/// `warp pure` is not rebuilt: that takes ~540 s (warp_pure_runs_hello_world)
fn lane(command: &str) -> std::process::Command {
	static SELF_BUILT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
	if command == "self" {
		SELF_BUILT.get_or_init(build_self_compiler);
	}
	let mut lane = crate::common::warp_command();
	lane.args(["--no-ask", command]).env("WARP_NO_WINDOW", "1").env("WARP_HINTS", "0");
	lane
}

fn build_self_compiler() {
	let built = crate::common::warp_command()
		.args(["rebuild", "self"])
		.env("WARP_TIME_LIMIT", SELF_BUILD_TIME_LIMIT_SECONDS)
		// a build for this run only: wasm-opt on the stages costs more than it saves (src/modules/binaryen.rs)
		.env("WARP_WASM_OPT", "0")
		.output()
		.expect("warp runs");
	assert!(built.status.success(), "warp rebuild self: {}", String::from_utf8_lossy(&built.stderr));
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
	let printed = said_by("self", "refused", "abs -3\n");
	assert!(printed.contains("outside the core: "), "{printed}");
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
	let said = pure_said("refused", "abs -3\n");
	assert!(said.contains("outside the core: the operator ‖ with nothing before it"), "{said}");
}

/// A library word compiles through the pure lane (card pure-not-a-list)
#[test]
fn warp_pure_runs_a_library_word() {
	let said = pure_said("library_word", "print sum [1, 2]\n");
	assert!(said.lines().next() == Some("3"), "{said}");
}

/// `warp self <file>`: compiler.warp compiled by itself (the pure lane) compiles the program (card self-hello)
#[test]
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
