//! A diagnostic of a used file names that file (card warning-file): `at used.warp:1:17`, not `at 1:17`, which reads as
//! a position in the program

#[test]
fn a_warning_in_a_used_file_names_the_file() {
	let program = concat!(env!("CARGO_MANIFEST_DIR"), "/probes/warning_file/main.warp");
	let run = crate::common::warp_command().env("WARP_NO_WINDOW", "1").arg(program).output().expect("warp runs");
	let warnings = String::from_utf8_lossy(&run.stderr);
	assert!(warnings.contains("names nothing"), "{warnings}");
	assert!(warnings.contains("warning_file/used.warp:1:17"), "{warnings}");
}
