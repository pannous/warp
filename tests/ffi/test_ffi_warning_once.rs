//! `import graphics` warns about missing FFI signatures once, not once per pass


#[test]
fn the_ffi_warning_is_printed_once() {
	let output = crate::common::warp_command().arg("import graphics; 1").output().expect("warp runs");
	let text = String::from_utf8_lossy(&output.stderr).to_string() + &String::from_utf8_lossy(&output.stdout);
	let warnings: Vec<&str> = text.lines().filter(|line| line.to_lowercase().contains("warning")).collect();
	let mut unique = warnings.clone();
	unique.dedup();
	assert_eq!(warnings, unique, "repeated warnings:\n{text}");
}
