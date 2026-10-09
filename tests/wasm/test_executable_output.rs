// Card executable-exit: an executable prints its value only when there is one worth showing (no trailing ø after a
// final print, a loop or an if without else), as `warp <file>` does; a trap's backtrace names main
use std::path::PathBuf;

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");

/// `warp build --exe` of `source` as <name>.warp, run: its stdout and stderr
fn executable_output(name: &str, source: &str) -> (String, String) {
	let source_path = PathBuf::from(OUTPUT_DIRECTORY).join(format!("{name}.warp"));
	std::fs::write(&source_path, source).unwrap();
	let build = crate::common::warp_command().env("WARP_RUNTIME_STUB", crate::common::runtime_stub()).args(["build", "--exe"]).arg(&source_path).output().unwrap();
	assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
	let run = std::process::Command::new(source_path.with_extension(std::env::consts::EXE_EXTENSION)).output().unwrap();
	(String::from_utf8_lossy(&run.stdout).into_owned(), String::from_utf8_lossy(&run.stderr).into_owned())
}

#[test]
fn test_executable_ending_in_a_print_prints_no_value() {
	assert_eq!(executable_output("ends_in_print", "print \"hello\"").0, "hello\n");
	assert_eq!(executable_output("ends_in_print_on_exit", "on exit { print \"bye\" }\nprint \"hello\"").0, "hello\nbye\n");
}

#[test]
fn test_executable_shows_no_empty_value() {
	assert_eq!(executable_output("ends_in_loop", "for i in 1..3 { print i }").0, "1\n2\n");
	assert_eq!(executable_output("ends_in_if", "x = 3\nif x > 5 { print \"big\" }").0, "");
	assert_eq!(executable_output("own_final_print", "xs = [1, 2]\nprint xs").0, "[1 2]\n");
}

#[test]
fn test_executable_still_shows_its_value() {
	assert_eq!(executable_output("ends_in_value", "x = 3\nif x > 1 { x * 2 }").0, "6\n");
	assert_eq!(executable_output("ends_in_list", "[1, 2, 3].map(x => x * 10)").0, "[10 20 30]\n");
}

#[test]
fn test_executable_trap_names_main() {
	let (_, stderr) = executable_output("trap_in_main", "f(x) := x / 0\nf(3)");
	assert!(stderr.contains("!main"), "{stderr}");
}
