// card sandbox-exec: `warp --sandbox` refuses every word that starts a process, writes a file or reaches the network
// with a loud error naming the flag, at compile time where the word shows and at run time where it hides behind a
// library word (`exec sh"…"` is os.exec through std_io); a pure program runs as always
use crate::common::warp_command;

/// What `warp --sandbox <code>` prints and whether it succeeded
fn sandboxed_run(code: &str) -> (String, bool) {
	let output = warp_command()
		.args(["--no-ask", "--sandbox", code])
		.env("WARP_HINTS", "0")
		.env("WARP_NO_WINDOW", "1")
		.output()
		.expect("warp runs");
	let printed = format!(
		"{}{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
	(printed, output.status.success())
}

fn refused(code: &str, capability: &str) {
	let (printed, succeeded) = sandboxed_run(code);
	assert!(
		!succeeded
			&& printed.contains("capability denied")
			&& printed.contains(capability)
			&& printed.contains("--sandbox"),
		"{code}: {printed}"
	);
}

#[test]
fn a_sandboxed_program_starts_no_process() {
	refused("exec sh\"echo SHELL-RAN\"", "process");
	refused("use os; exec(sh\"echo SHELL-RAN\")", "process");
	assert!(!sandboxed_run("exec sh\"echo SHELL-RAN\"").0.contains("\"SHELL-RAN\""));
}

#[test]
fn a_sandboxed_program_writes_no_file() {
	let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("sandbox_refusals_written.txt");
	let _ = std::fs::remove_file(&file);
	let path = file.display();
	refused(&format!("use file; write(\"{path}\", \"x\")"), "file-write");
	refused(&format!("use file; file.append(\"{path}\", \"x\")"), "file-write");
	refused(&format!("use file; write_bytes(\"{path}\", [1, 2])"), "file-write");
	assert!(!file.exists(), "{path} was written");
}

#[test]
fn a_sandboxed_program_reaches_no_network() {
	refused("fetch \"http://localhost:1\"", "network");
	refused("use net; post(\"http://localhost:1\", \"x\")", "network");
}

#[test]
fn a_sandboxed_pure_program_still_runs() {
	let (printed, succeeded) =
		sandboxed_run("fib(n) := if n < 2 then n else fib(n-1) + fib(n-2); [1, 2, 3].map(x => x * fib(10))");
	assert!(
		succeeded && printed.contains("55") && printed.contains("165"),
		"{printed}"
	);
}
