//! `warp pure <file>`: lib/compiler/compiler.warp, the warp compiler as one warp program, compiles the program and its
//! module runs (card warp-pure, src/compiler/standalone.rs)
const HELLO: &str = "print(\"hello world\")\n";

#[test]
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
