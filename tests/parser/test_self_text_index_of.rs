//! card index-of-bytes: the warp runtime reports text positions in graphemes, like indexing and slicing.
use std::path::PathBuf;

const PROGRAM: &str = r#"[
    "a·b(c)".index_of("(")
    "q\u{301}x".index_of("x")
    "a👍🏽b".index_of("b")
]"#;

#[test]
fn text_index_of_counts_graphemes_self_hosted() {
	let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
	let output = crate::common::warp_command()
		.env("WARP_SELF_HOSTED", "pure")
		.env("WARP_MODULE_CACHE", scratch.join("self_hosted_stage_cache"))
		.arg(PROGRAM)
		.output()
		.expect("warp runs");
	let stdout = String::from_utf8_lossy(&output.stdout);
	assert!(stdout.lines().any(|line| line.trim_start_matches('»').trim() == "[4 2 3]"), "{stdout}");
}
