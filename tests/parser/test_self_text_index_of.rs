//! card index-of-bytes, decision byte-index-of: a text names the unit it counts a part's place in. `find` counts
//! characters (graphemes) from 1, `byte_index_of` bytes from 0; a bare `index_of` on a text counts as find does and
//! warns; a missing part is an error.
use std::path::PathBuf;

/// The output of the program in the pure lane, its warnings included
fn pure_output(code: &str) -> String {
	let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
	let output = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", "pure")
		.env("WARP_MODULE_CACHE", scratch.join("self_hosted_stage_cache"))
		.arg(code)
		.output()
		.expect("warp runs");
	String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr)
}

fn prints(code: &str, value: &str) {
	let output = pure_output(code);
	assert!(output.lines().any(|line| line.trim_start_matches('»').trim() == value), "{code}: {output}");
}

#[test]
fn text_find_counts_graphemes_from_one() {
	prints(r#"[find("a·b(c)", "("), "q\u{301}x".find("x"), "a👍🏽b".find("b")]"#, "[4 2 3]");
}

#[test]
fn text_byte_index_of_counts_bytes_from_zero() {
	prints(r#"["a·b(c)".byte_index_of("("), byte_index_of("a👍🏽b", "b")]"#, "[4 9]");
}

#[test]
fn text_find_answers_by_the_declared_count() {
	prints(r#"t = "a👍🏽b"; position p = t.find("b"); i: index = t.find("b"); [p, i]"#, "[3 2]");
}

/// decision index-units: a declared index or position holds a value counted its way (lib/compiler/index_positions.warp)
#[test]
fn a_declared_index_or_position_holds_its_count() {
	let program = format!("{}/probes/index_of_bytes/declared_counts.warp", env!("CARGO_MANIFEST_DIR"));
	prints(&program, "[1 2 3 2 1]");
}

#[test]
fn a_missing_text_part_is_an_error() {
	let output = pure_output(r#""abc".find("z")"#);
	assert!(output.contains(r#""z" is not in"#), "{output}");
}

#[test]
fn text_index_of_counts_as_find_and_warns() {
	let output = pure_output(r#"t = "a👍🏽b"; t.index_of("b")"#);
	assert!(output.lines().any(|line| line.trim_start_matches('»').trim() == "3"), "{output}");
	assert!(output.contains("byte_index_of"), "the warning names byte_index_of: {output}");
}
