// Hints are colored only on a terminal (and without NO_COLOR): piped into a file or an editor's output panel they are
// plain text, never raw escape codes (card hint-colors)
#![cfg(feature = "native")]

#[test]
fn a_hint_on_piped_stderr_has_no_escape_codes() {
	let output = crate::common::warp_command().args(["--no-ask", "eval", "x = 'hello'; x"]).output().expect("warp runs");
	let errors = String::from_utf8_lossy(&output.stderr);
	assert!(errors.contains("hint 1:5: prefer \"hello\" over 'hello'"), "{errors}");
	assert!(!errors.contains('\x1b'), "{errors:?}");
}
