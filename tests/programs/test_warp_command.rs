// The CLI tests run this checkout's warp binary through common::warp_command, never the shared target/debug/warp directly
#![cfg(feature = "native")]

#[test]
fn warp_command_runs_this_checkouts_binary() {
	let output = crate::common::warp_command().arg("version").output().expect("warp runs");
	assert!(String::from_utf8_lossy(&output.stdout).contains(env!("CARGO_PKG_VERSION")));
}
