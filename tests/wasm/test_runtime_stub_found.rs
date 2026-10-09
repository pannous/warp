// `warp build` just works (issue #10) with the prebuilt warp-runtime shipped next to warp. An installed warp never builds
// the stub itself (user 2026-10-09: "It should never be built by the script itself. It should already be prebuilt."):
// without one it says where warp-runtime belongs
#![cfg(feature = "native")]
use std::path::PathBuf;

#[test]
fn build_finds_or_builds_the_runtime_stub_by_itself() {
	let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stubless");
	std::fs::create_dir_all(&folder).unwrap();
	let warp = folder.join("warp");
	let _ = std::fs::remove_file(&warp);
	std::fs::copy(crate::common::warp_command().get_program(), &warp).unwrap();
	let stub = folder.join("warp-runtime");
	let _ = std::fs::remove_file(&stub);
	let source = folder.join("seven.warp");
	std::fs::write(&source, "3 + 4").unwrap();
	let executable = source.with_extension(std::env::consts::EXE_EXTENSION);
	let _ = std::fs::remove_file(&executable);
	let build = || std::process::Command::new(&warp).env_remove("WARP_RUNTIME_STUB").args(["build", "--exe"]).arg(&source).output().unwrap();

	let stubless = build();
	let complaint = String::from_utf8_lossy(&stubless.stderr);
	assert!(!stubless.status.success() && complaint.contains("warp-runtime ships next to warp"), "{complaint}");
	assert!(!complaint.contains("building the runtime stub"), "an installed warp never builds the stub: {complaint}");

	std::fs::copy(crate::common::runtime_stub(), &stub).unwrap();
	let built = build();
	assert!(built.status.success(), "{}", String::from_utf8_lossy(&built.stderr));
	let run = std::process::Command::new(&executable).output().unwrap();
	assert_eq!(String::from_utf8_lossy(&run.stdout), "7\n", "{}", String::from_utf8_lossy(&run.stderr));
}
