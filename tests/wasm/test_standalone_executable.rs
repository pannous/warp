use std::path::PathBuf;
use std::process::Output;
use warp_runtime::standalone::{embedded_machine_code, with_machine_code, without_machine_code};

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");

/// `warp build --exe` of `source`, written to <name>.warp: the build's output and the executable's path. Without a
/// warp-runtime stub next to the test's warp the executable is a copy of warp itself, which runs what it carries too.
fn build_executable(name: &str, source: &str) -> (Output, PathBuf) {
	let source_path = PathBuf::from(OUTPUT_DIRECTORY).join(format!("{name}.warp"));
	std::fs::write(&source_path, source).unwrap();
	let executable = source_path.with_extension("exe");
	let _ = std::fs::remove_file(&executable);
	let build = crate::common::warp_command().env_remove("WARP_RUNTIME_STUB").args(["build", "--exe"]).arg(&source_path).output().unwrap();
	(build, executable)
}

fn text(bytes: &[u8]) -> String {
	String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn test_build_exe_runs_the_program_and_prints_its_value() {
	let (build, executable) = build_executable("standalone_value", "print \"hello\"\nsquare(x) := x * x\nsquare(7) + floor(sqrt(16))");
	assert!(build.status.success(), "{}", text(&build.stderr));
	let run = std::process::Command::new(&executable).output().unwrap();
	assert!(run.status.success(), "{}", text(&run.stderr));
	assert_eq!(text(&run.stdout), "hello\n53\n");
}

#[test]
fn test_build_exe_prints_lists_as_print_does() {
	let (build, executable) = build_executable("standalone_list", "[1, 2, 3].map(x => x * 10)");
	assert!(build.status.success(), "{}", text(&build.stderr));
	let run = std::process::Command::new(&executable).output().unwrap();
	assert_eq!(text(&run.stdout), "[10 20 30]\n", "{}", text(&run.stderr));
}

#[test]
fn test_build_exe_refuses_imports_the_runtime_lacks() {
	let (build, executable) = build_executable("standalone_fetch", "fetch \"https://example.com\"");
	assert!(!build.status.success());
	assert!(text(&build.stderr).contains("host.fetch"), "{}", text(&build.stderr));
	assert!(!executable.exists());
}

#[test]
fn test_machine_code_rides_at_the_end_of_the_executable() {
	let executable = with_machine_code(b"runtime", b"machine code");
	assert_eq!(without_machine_code(&executable), b"runtime");
	// carrying another program replaces the first
	let rebuilt = with_machine_code(&executable, b"other");
	assert_eq!(without_machine_code(&rebuilt), b"runtime");
	let path = PathBuf::from(OUTPUT_DIRECTORY).join("carried_machine_code.bin");
	std::fs::write(&path, &rebuilt).unwrap();
	assert_eq!(embedded_machine_code(&path).as_deref(), Some(&b"other"[..]));
	std::fs::write(&path, b"runtime").unwrap();
	assert_eq!(embedded_machine_code(&path), None);
}

/// macOS: the executable is signed ad hoc and passes strict verification, its program inside the __LINKEDIT segment
#[cfg(target_os = "macos")]
#[test]
fn test_build_exe_signs_cleanly_on_macos() {
	let (build, executable) = build_executable("standalone_signed", "6 * 7");
	assert!(build.status.success(), "{}", text(&build.stderr));
	let verify = std::process::Command::new("codesign").args(["--verify", "--strict", "--verbose=2"]).arg(&executable).output().unwrap();
	assert!(verify.status.success(), "{}", text(&verify.stderr));
	let run = std::process::Command::new(&executable).output().unwrap();
	assert_eq!(text(&run.stdout), "42\n", "{}", text(&run.stderr));
	// built again from the signed executable as its runtime: still one program, still signed cleanly
	let rebuilt = without_machine_code(&std::fs::read(&executable).unwrap());
	assert!(embedded_machine_code(&executable).is_some());
	assert!(rebuilt.len() < std::fs::metadata(&executable).unwrap().len() as usize);
}
