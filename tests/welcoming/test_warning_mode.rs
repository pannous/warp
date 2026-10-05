//! Warnings are reported and compilation continues, unless warnings are errors:
//! `use strict` in wasp, `--strict` on the command line, `diagnostic::with_warning_mode` from Rust.
//! `warning(message)` reports at runtime: "" as a warning, an Error value when warnings are errors.

use warp::diagnostic::{take_runtime_warnings, with_warning_mode, WarningMode};
use warp::wasm_emitter::eval;
use warp::{error, is, Node};

#[test]
fn a_runtime_warning_is_reported_and_yields_nothing() {
	take_runtime_warnings();
	is!("\"a\" + warning(\"careful\") + \"b\"", "ab");
	assert_eq!(take_runtime_warnings(), vec!["careful".to_string()]);
}

#[test]
fn use_strict_turns_a_runtime_warning_into_an_error() {
	is!("use strict; \"a\" + warning(\"careful\") + \"b\"", error("careful"));
}

#[test]
fn the_rust_setting_turns_a_runtime_warning_into_an_error() {
	let strict = with_warning_mode(WarningMode::Error, || eval("\"a\" + warning(\"careful\")"));
	assert_eq!(strict, error("careful"));
	assert_eq!(eval("\"a\" + warning(\"careful\")"), Node::Text("a".into()), "the setting is scoped");
	take_runtime_warnings();
}

#[test]
fn use_strict_turns_a_lint_warning_into_an_error() {
	is!("7 % -2", 1);
	assert!(matches!(eval("use strict; 7 % -2"), Node::Error(_)));
}

#[test]
fn the_strict_flag_turns_warnings_into_errors() {
	let run = |args: &[&str]| {
		let output = crate::common::warp_command().args(args).output().expect("warp runs");
		String::from_utf8_lossy(&output.stdout).to_string() + &String::from_utf8_lossy(&output.stderr)
	};
	assert!(run(&["\"a\" + warning(\"careful\")"]).contains("warning: careful"));
	assert!(run(&["--strict", "\"a\" + warning(\"careful\")"]).contains("Error(\"careful\")"));
}
