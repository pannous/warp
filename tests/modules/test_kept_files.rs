// card speed-module-cache: a used file's lowering (the Rust lane) and its parse (the pure lane) are kept by its source,
// so a rebuild redoes only the files that changed: a run from the kept ones prints what a run without them prints, and
// a run after an edit of the file prints the edited value
#![cfg(feature = "native")]
use std::path::{Path, PathBuf};

/// A folder for one test's program, the file it uses and its own cache
fn folder(name: &str) -> PathBuf {
	let folder = crate::common::scratch_directory(name);
	std::fs::create_dir_all(folder.join("cache")).unwrap();
	std::fs::write(folder.join("program.warp"), "use \"helper.warp\"\nprint(answer())\n").unwrap();
	std::fs::write(folder.join("helper.warp"), "answer() := 6 * 7\n").unwrap();
	folder
}

/// What `warp program.warp` prints with the cache setting, in the lane WARP_SELF_HOSTED names (none: the Rust lane)
fn printed(folder: &Path, cache: &str, lane: Option<&str>) -> String {
	let mut command = crate::common::warp_command();
	command.args(["--no-ask", "program.warp"]).current_dir(folder).env(warp::run::module_cache::CACHE_VARIABLE, cache);
	if let Some(lane) = lane {
		command.env("WARP_SELF_HOSTED", lane);
	}
	let run = command.output().unwrap();
	assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
	String::from_utf8_lossy(&run.stdout).into_owned()
}

fn kept_runs_see_edits(name: &str, lane: Option<&str>) {
	let folder = folder(name);
	let cache = folder.join("cache").display().to_string();
	assert!(printed(&folder, "off", lane).contains("42"));
	assert!(printed(&folder, &cache, lane).contains("42"), "the cold run");
	assert!(printed(&folder, &cache, lane).contains("42"), "the run from the kept file");
	std::fs::write(folder.join("helper.warp"), "answer() := 6 * 8\n").unwrap();
	assert!(printed(&folder, &cache, lane).contains("48"), "the run after the edit of the used file");
}

#[test]
fn a_kept_used_file_gives_its_value_and_sees_its_edits() {
	kept_runs_see_edits("kept_files_rust", None);
}

#[test]
fn a_kept_used_file_gives_its_value_and_sees_its_edits_in_the_pure_lane() {
	kept_runs_see_edits("kept_files_pure", Some("pure-only"));
}
