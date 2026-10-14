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

/// Every build names the branch it was built from too, so a stale install is obvious (card version-banner-version):
/// `debug build 75d67b068 on main · built 2026-10-10 13:03`
#[test]
fn version_names_the_branch() {
	let output = crate::common::warp_command().arg("version").output().expect("warp runs");
	let stderr = String::from_utf8_lossy(&output.stderr);
	let branch =
		std::process::Command::new("git").args(["rev-parse", "--abbrev-ref", "HEAD"]).output().expect("git runs");
	let branch = String::from_utf8_lossy(&branch.stdout).trim().to_string();
	if branch != "HEAD" {
		assert!(stderr.contains(&format!(" on {branch} · built 20")), "{stderr}");
	}
}

/// A program whose first word names a command is still a program when that word is assigned (card top-level):
/// `warp 'lower = "abc"; lower'` printed "no module needed: Error(…)"
#[test]
fn an_assigned_command_word_is_a_variable() {
	for (program, expected) in [("lower = \"abc\"; lower", "abc"), ("parse = 3; parse", "3"), ("eval := 4; eval", "4")]
	{
		let output = crate::common::warp_command()
			.arg(program)
			.env("WARP_NO_WINDOW", "1")
			.env("WARP_HINTS", "0")
			.output()
			.expect("warp runs");
		let stdout = String::from_utf8_lossy(&output.stdout);
		assert!(stdout.contains(expected) && !stdout.contains("Error"), "{program}: {stdout}");
	}
}
