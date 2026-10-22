//! An error and a warning of a program file name the file, `at file:line:column`, above the line they point at
//! (card error-names: "Error messages must also print the file")

fn run(program: &str) -> String {
	let path = concat!(env!("CARGO_MANIFEST_DIR"), "/probes/error_names/").to_string() + program;
	let run = crate::common::warp_command().env("WARP_NO_WINDOW", "1").args(["--no-ask", &path]).output().unwrap();
	format!("{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr))
}

#[test]
fn a_runtime_error_names_its_file() {
	let said = run("index_error.warp");
	assert!(said.contains("index out of range: 5 not in 1…2 at "), "{said}");
	assert!(said.contains("probes/error_names/index_error.warp:4:"), "{said}");
	assert!(said.contains("  4 | print xs#5"), "{said}");
}

#[test]
fn a_warning_names_its_file() {
	let said = run("warning.warp");
	assert!(said.contains("probes/error_names/warning.warp:4:12; fix: ..<"), "{said}");
	assert!(said.contains("  4 | for i in 1 upto 4 { x += i }"), "{said}");
}
