// A debug build embeds the commit it was built from as WARP_COMMIT for `warp version`: asking git at run time named
// the checkout's current HEAD instead (card version-prints). Release builds skip it, so a commit doesn't rebuild them.
use std::process::Command;

fn git(arguments: &[&str]) -> Option<String> {
	let output = Command::new("git").args(arguments).output().ok().filter(|output| output.status.success())?;
	Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
	println!("cargo:rerun-if-changed=build.rs");
	if std::env::var("PROFILE").as_deref() != Ok("debug") {
		return;
	}
	// logs/HEAD grows on every commit, checkout and reset, also of a worktree (its own git dir)
	// (a missing file would rerun this on every build)
	if let Some(head_log) = git(&["rev-parse", "--git-path", "logs/HEAD"]).filter(|path| std::path::Path::new(path).exists()) {
		println!("cargo:rerun-if-changed={head_log}");
	}
	if let Some(commit) = git(&["rev-parse", "--short=9", "HEAD"]) {
		println!("cargo:rustc-env=WARP_COMMIT={commit}");
	}
}
