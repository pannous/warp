//! `use script` (alias `use script mode`, decision use-script): `use relaxed` plus the scripting words of os, file and
//! net without further `use`, with `command`, `arguments`, `cwd`, `glob`, `stdin` and `input`; `exit n` is the exit
//! status of inline code too
#![cfg(feature = "native")]
use crate::common::{printed, warp_command};
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasm_emitter::eval;

fn script(code: &str) -> String {
	printed(&format!("use script\n{code}"))
}

#[test]
fn a_script_shows_no_hints() {
	set_hint_mode(HintMode::Always);
	for pragma in ["use script", "use script mode"] {
		assert!(capture_hints(|| eval(&format!("{pragma}\n2**3"))).1.is_empty(), "{pragma}");
	}
}

#[test]
fn a_script_has_the_scripting_words() {
	assert_eq!(script("print command(\"echo hi\")"), "hi\n");
	assert_eq!(printed("use script mode\nprint command(\"echo hi\")"), "hi\n");
	assert_eq!(script("print glob(\"Cargo.t*\")"), "[\"Cargo.toml\"]\n");
	let folder = std::env::current_dir().unwrap();
	assert_eq!(script("print cwd"), format!("{}\n", folder.display()));
	assert_eq!(script("print exists(\"Cargo.toml\")"), "yes\n");
}

#[test]
fn a_script_reads_its_input() {
	let mut run = warp_command()
		.args(["--no-ask", "use script\nprint input(\"name? \")"])
		.stdin(std::process::Stdio::piped())
		.stdout(std::process::Stdio::piped())
		.spawn()
		.unwrap();
	use std::io::Write;
	run.stdin.take().unwrap().write_all(b"bob\n").unwrap();
	let output = run.wait_with_output().unwrap();
	assert_eq!(String::from_utf8_lossy(&output.stdout), "name? bob\n");
}

#[test]
fn inline_code_exits_with_its_exit_status() {
	let output = warp_command().args(["--no-ask", "print 1; exit 3"]).output().unwrap();
	assert_eq!(output.status.code(), Some(3));
	assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
}
