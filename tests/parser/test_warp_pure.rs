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

/// `warp self <file>`: compiler.warp compiled by itself (the pure lane) compiles the program (card self-hello)
#[test]
fn warp_self_runs_hello_world() {
	let folder = crate::common::scratch_directory("warp_self");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join("hello.warp");
	std::fs::write(&program, HELLO).unwrap();
	let run = crate::common::warp_command()
		.args(["--no-ask", "self"])
		.arg(&program)
		.env("WARP_NO_WINDOW", "1")
		.env("WARP_HINTS", "0")
		.output()
		.expect("warp runs");
	let printed = String::from_utf8_lossy(&run.stdout);
	assert!(printed.lines().next() == Some("hello world"), "{printed}\n{}", String::from_utf8_lossy(&run.stderr));
}

/// What `warp self` prints running the code (card self-compiled-parity)
fn printed_by_warp_self(name: &str, code: &str) -> String {
	let folder = crate::common::scratch_directory("warp_self");
	std::fs::create_dir_all(&folder).unwrap();
	let program = folder.join(format!("{name}.warp"));
	std::fs::write(&program, code).unwrap();
	let run = crate::common::warp_command()
		.args(["--no-ask", "self"])
		.arg(&program)
		.env("WARP_NO_WINDOW", "1")
		.env("WARP_HINTS", "0")
		.output()
		.expect("warp runs");
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
}

/// Loops and text concatenation under `warp self`: analyzer.warp's loop kind of a body ending in a non-group (`{ 2 }`),
/// node_matches.warp's `True:` arm of node_text (cards match-bool-arm, self-compiled-parity)
#[test]
fn warp_self_runs_loops_and_texts() {
	let code = "n = 0\nwhile n < 3 { n += 1 }\ns = \"a\"\ns += \"b\"\nprint s + n";
	let printed = printed_by_warp_self("loops_and_texts", code);
	assert!(printed.lines().any(|line| line == "ab3"), "{printed}");
}

/// A program outside the emitter's core fails under `warp self` with the refusal, not a bare trap (card
/// compiler-refusal-loud)
#[test]
fn warp_self_names_the_refusal() {
	let printed = printed_by_warp_self("refused", "f(x) := x.length(); f(\"abc\")");
	assert!(printed.contains("outside the core: "), "{printed}");
}
