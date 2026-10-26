//! `warp pure <command>` and `warp self <command>`: warp's commands written in warp (core/commands.warp), the
//! same ones `wasmtime warp-self.wasm <command>` runs (card wasm-commands, notes/standalone.md)
use super::test_warp_pure::lane;
use std::io::Write;

const LANES: [&str; 2] = ["pure", "self"];

/// What `warp <lane> <arguments…>` printed, its errors after its output
fn said(command: &str, arguments: &[&str]) -> String {
	let run = lane(command).args(arguments).output().expect("warp runs");
	String::from_utf8_lossy(&run.stdout).to_string() + &String::from_utf8_lossy(&run.stderr)
}

#[test]
fn help_lists_the_commands() {
	for command in LANES {
		let help = said(command, &["help"]);
		for listed in ["warp parse", "warp repl", "need the native warp"] {
			assert!(help.contains(listed), "{command} help: {help}");
		}
	}
}

#[test]
fn parse_shows_prefix_groups() {
	for command in LANES {
		let parsed = said(command, &["parse", "1+2*x"]);
		assert!(parsed.lines().any(|line| line == "(+ 1 (* 2 x))"), "{command} parse: {parsed}");
	}
}

#[test]
fn a_host_command_names_the_native_warp() {
	for command in LANES {
		let refused = said(command, &["serve"]);
		assert!(refused.contains("needs the native warp's host"), "{command} serve: {refused}");
	}
}

#[test]
fn repl_keeps_declarations_for_the_next_lines() {
	for command in LANES {
		let mut repl = lane(command)
			.arg("repl")
			.stdin(std::process::Stdio::piped())
			.stdout(std::process::Stdio::piped())
			.stderr(std::process::Stdio::piped())
			.spawn()
			.expect("warp runs");
		repl.stdin.take().unwrap().write_all(b"double(a: int) := a * 2\nx = 21\ndouble(x)\n").unwrap();
		let run = repl.wait_with_output().unwrap();
		let shown = String::from_utf8_lossy(&run.stdout).to_string() + &String::from_utf8_lossy(&run.stderr);
		assert!(shown.lines().any(|line| line.ends_with("42")), "{command} repl: {shown}");
	}
}
