// `warp build` just works (issue #10): without WARP_RUNTIME_STUB and without a warp-runtime next to warp, warp builds
// the stub from its own source checkout once and uses it, instead of failing with "no runtime stub"
#![cfg(feature = "native")]
use std::path::PathBuf;

#[test]
fn build_finds_or_builds_the_runtime_stub_by_itself() {
	let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stubless");
	std::fs::create_dir_all(&folder).unwrap();
	let warp = folder.join("warp");
	let _ = std::fs::remove_file(&warp);
	std::fs::copy(crate::common::warp_command().get_program(), &warp).unwrap();
	let _ = std::fs::remove_file(folder.join("warp-runtime"));
	let source = folder.join("seven.warp");
	std::fs::write(&source, "3 + 4").unwrap();
	let executable = source.with_extension(std::env::consts::EXE_EXTENSION);
	let _ = std::fs::remove_file(&executable);
	let build = std::process::Command::new(&warp).env_remove("WARP_RUNTIME_STUB").args(["build", "--exe"]).arg(&source).output().unwrap();
	assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
	let run = std::process::Command::new(&executable).output().unwrap();
	assert_eq!(String::from_utf8_lossy(&run.stdout), "7\n", "{}", String::from_utf8_lossy(&run.stderr));
}
