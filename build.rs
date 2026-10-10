// Every build embeds the commit and branch it was built from as WARP_COMMIT and WARP_BRANCH for `warp version` and the
// console banner, so a stale install is obvious (cards version-prints, version-banner-version): asking git at run time
// named the checkout's current HEAD instead.
use std::process::Command;

const DETACHED_HEAD: &str = "HEAD";

fn git(arguments: &[&str]) -> Option<String> {
	let output = Command::new("git").args(arguments).output().ok().filter(|output| output.status.success())?;
	Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn main() {
	println!("cargo:rerun-if-changed=build.rs");
	// logs/HEAD grows on every commit, checkout and reset, also of a worktree (its own git dir)
	// (a missing file would rerun this on every build)
	if let Some(head_log) = git(&["rev-parse", "--git-path", "logs/HEAD"]).filter(|path| std::path::Path::new(path).exists()) {
		println!("cargo:rerun-if-changed={head_log}");
	}
	if let Some(commit) = git(&["rev-parse", "--short=9", "HEAD"]) {
		println!("cargo:rustc-env=WARP_COMMIT={commit}");
	}
	if let Some(branch) = git(&["rev-parse", "--abbrev-ref", "HEAD"]).filter(|branch| branch != DETACHED_HEAD) {
		println!("cargo:rustc-env=WARP_BRANCH={branch}");
	}
}
