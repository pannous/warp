//! card build-should: `warp build app.warp` writes the executable app and the WebAssembly module app.wasm, `--exe` too
//! (accepted, redundant), `--wasm` only the module (Interviewer decision 2026-10-13)
use std::path::PathBuf;

const OUTPUT_DIRECTORY: &str = env!("CARGO_TARGET_TMPDIR");

/// `warp build <flags> <name>.warp` of `6 * 7`: whether the executable and the module are there afterwards
fn built(name: &str, flags: &[&str]) -> (bool, bool) {
	let source_path = PathBuf::from(OUTPUT_DIRECTORY).join(format!("{name}.warp"));
	std::fs::write(&source_path, "6 * 7").unwrap();
	let (executable, module) =
		(source_path.with_extension(std::env::consts::EXE_EXTENSION), source_path.with_extension("wasm"));
	let _ = std::fs::remove_file(&executable);
	let _ = std::fs::remove_file(&module);
	let build = crate::common::warp_command()
		.env("WARP_RUNTIME_STUB", crate::common::runtime_stub())
		.arg("build")
		.args(flags)
		.arg(&source_path)
		.output()
		.unwrap();
	assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
	(executable.exists(), module.exists())
}

#[test]
fn build_writes_the_executable_and_the_module() {
	assert_eq!(built("build_both", &[]), (true, true));
	let module = PathBuf::from(OUTPUT_DIRECTORY).join("build_both.wasm");
	let run = crate::common::warp_command().arg(&module).output().unwrap();
	assert!(String::from_utf8_lossy(&run.stdout).contains("42"), "{}", String::from_utf8_lossy(&run.stderr));
}

#[test]
fn build_exe_writes_both_and_build_wasm_only_the_module() {
	assert_eq!(built("build_exe_too", &["--exe"]), (true, true));
	assert_eq!(built("build_wasm_only", &["--wasm"]), (false, true));
}
