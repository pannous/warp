// The CLI tests run this checkout's warp binary through common::warp_command, never the shared target/debug/warp directly
#![cfg(feature = "native")]

#[test]
fn warp_command_runs_this_checkouts_binary() {
	let output = crate::common::warp_command().arg("version").output().expect("warp runs");
	assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
}

/// A debug build names its commit and build time on stderr (cards g_oMw8, g_oM-0): `debug build 75d67b068 · built 2026-10-09 14:32`
#[test]
fn debug_build_names_its_commit() {
	let output = crate::common::warp_command().arg("version").output().expect("warp runs");
	let stderr = String::from_utf8_lossy(&output.stderr);
	let commit = std::process::Command::new("git").args(["rev-parse", "--short=9", "HEAD"]).output().expect("git runs");
	let commit = String::from_utf8_lossy(&commit.stdout).trim().to_string();
	assert!(stderr.contains(&format!("debug build {commit}")), "{stderr}");
	assert!(stderr.contains(" · built 20"), "{stderr}");
}
