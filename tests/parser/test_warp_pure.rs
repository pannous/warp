//! `warp pure <file>`: lib/compiler/compiler.warp, the warp compiler as one warp program, compiles the program and its
//! module runs (card warp-pure, src/compiler/standalone.rs)
const HELLO: &str = "print(\"hello world\")\n";

#[test]
#[ignore = "slow: compiles all of compiler.warp, 540 s of the 600 s test limit at opt-level 2 (cloud, 2026-10-21); run with --ignored"]
fn warp_pure_runs_hello_world() {
	let folder = crate::common::scratch_directory("warp_pure");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join("hello.warp");
	std::fs::write(&program, HELLO).unwrap();
	let run = crate::common::warp_command()
		.args(["--no-ask", "pure"])
		.arg(&program)
		.env("WARP_NO_WINDOW", "1")
		.env("WARP_HINTS", "0")
		.output()
		.expect("warp runs");
	let printed = String::from_utf8_lossy(&run.stdout);
	assert!(printed.lines().next() == Some("hello world"), "{printed}\n{}", String::from_utf8_lossy(&run.stderr));
}

/// What `warp pure` says of the program `code`: its output and errors
fn pure_said(name: &str, code: &str) -> String {
	let folder = crate::common::scratch_directory("warp_pure");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join(format!("{name}.warp"));
	std::fs::write(&program, code).unwrap();
	let run = crate::common::warp_command()
		.args(["--no-ask", "pure"])
		.arg(&program)
		.env("WARP_NO_WINDOW", "1")
		.env("WARP_HINTS", "0")
		.output()
		.expect("warp runs");
	String::from_utf8_lossy(&run.stdout).to_string() + &String::from_utf8_lossy(&run.stderr)
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
