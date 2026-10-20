// card cache-compiled-shader: `warp <file>` keeps the module a program compiles to (src/run/program_cache.rs), so its
// next run starts without compiling; a change to the program, to a file of its folder or to warp itself compiles again
#![cfg(feature = "native")]
use std::path::{Path, PathBuf};

/// A folder for one test's program and its own cache
fn folder(name: &str) -> PathBuf {
	let folder = crate::common::scratch_directory(name);
	std::fs::create_dir_all(folder.join("cache")).unwrap();
	folder
}

/// What `warp run program.warp` in the folder prints, with the folder's cache
fn printed(folder: &Path) -> String {
	run(folder).0
}

/// What `warp run program.warp` in the folder prints on stdout and on stderr, with the folder's cache
fn run(folder: &Path) -> (String, String) {
	run_with(folder, &[])
}

/// What `warp run program.warp` prints with these settings beside the folder's cache
fn run_with(folder: &Path, settings: &[(&str, &str)]) -> (String, String) {
	let run = crate::common::warp_command()
		.args(["--no-ask", "run", "program.warp"])
		.current_dir(folder)
		.env(warp::run::module_cache::CACHE_VARIABLE, folder.join("cache"))
		.envs(settings.iter().copied())
		.output()
		.unwrap();
	assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
	let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).trim().to_string();
	(text(&run.stdout), text(&run.stderr))
}

/// The programs the folder's cache holds
fn cached_programs(folder: &Path) -> usize {
	let extension = Some(std::ffi::OsStr::new(warp::run::program_cache::PROGRAM_EXTENSION));
	std::fs::read_dir(folder.join("cache"))
		.unwrap()
		.filter(|entry| entry.as_ref().unwrap().path().extension() == extension)
		.count()
}

#[test]
fn a_program_runs_again_from_its_cached_module() {
	let folder = folder("program_cache_again");
	std::fs::write(folder.join("program.warp"), "square(x) := x * x\nprint square(7)").unwrap();
	assert_eq!(printed(&folder), "49");
	assert_eq!(cached_programs(&folder), 1);
	assert_eq!(printed(&folder), "49");
	assert_eq!(cached_programs(&folder), 1);
	std::fs::write(folder.join("program.warp"), "square(x) := x * x\nprint square(8)").unwrap();
	assert_eq!(printed(&folder), "64");
	assert_eq!(cached_programs(&folder), 2);
}

#[test]
fn a_changed_file_of_the_folder_compiles_again() {
	let folder = folder("program_cache_folder");
	std::fs::write(folder.join("program.warp"), "use words\nprint bump(1)").unwrap();
	std::fs::write(folder.join("words.warp"), "bump(x) := x + 1").unwrap();
	assert_eq!(printed(&folder), "2");
	assert_eq!(cached_programs(&folder), 1);
	std::fs::write(folder.join("words.warp"), "bump(x) := x + 10").unwrap();
	assert_eq!(printed(&folder), "11");
}

// card program-cache-paths: a module looked for beyond the keyed folders is kept with its stamp
#[test]
fn a_module_looking_beyond_the_keyed_folders_is_kept_until_it_changes() {
	let folder = folder("program_cache_nested");
	std::fs::create_dir_all(folder.join("lib/include")).unwrap();
	std::fs::write(folder.join("program.warp"), "use outer\nprint bump(1)").unwrap();
	std::fs::write(folder.join("lib/outer.warp"), "use inner\nbump(x) := twice(x) + 1").unwrap();
	std::fs::write(folder.join("lib/include/inner.warp"), "twice(x) := 2 * x").unwrap();
	assert_eq!(printed(&folder), "3");
	assert_eq!(cached_programs(&folder), 1);
	std::fs::write(folder.join("lib/include/inner.warp"), "twice(x) := 20 * x").unwrap();
	assert_eq!(printed(&folder), "21");
}

// card program-cache-own-output: the key lists the folder's module files only, so a program's own output beside it
// (`save "x.png"`, `write("out.txt", …)`) does not compile it again
#[test]
fn a_file_the_program_writes_beside_it_keeps_its_cached_module() {
	let folder = folder("program_cache_own_output");
	std::fs::write(folder.join("program.warp"), "write(\"out.txt\", \"made\")\nprint 5").unwrap();
	assert_eq!(printed(&folder), "5");
	assert_eq!(printed(&folder), "5");
	assert_eq!(cached_programs(&folder), 1);
}

// card program-cache-paths: `use "./parts/x.warp"` is kept until that file changes, and a run from the kept module says
// the compile's warnings again
#[test]
fn a_module_used_by_its_path_and_a_warning_are_kept() {
	let folder = folder("program_cache_path");
	std::fs::create_dir_all(folder.join("parts")).unwrap();
	std::fs::write(folder.join("program.warp"), "use \"./parts/words.warp\"\nprint bump(Round)").unwrap();
	std::fs::write(folder.join("parts/words.warp"), "bump(x) := [x, 1]").unwrap();
	let (first, warned) = run(&folder);
	assert!(warned.contains("`Round` names nothing"), "{warned}");
	assert_eq!(run(&folder), (first, warned));
	assert_eq!(cached_programs(&folder), 1);
	std::fs::write(folder.join("parts/words.warp"), "bump(x) := [x, 2]").unwrap();
	assert!(printed(&folder).contains('2'));
}

// card run-settings-keep: a setting only the run reads (its time limit, a profile, the phase times) runs the kept module
#[test]
fn a_run_only_setting_runs_the_kept_module() {
	let folder = folder("program_cache_run_settings");
	std::fs::write(folder.join("program.warp"), "print 6 * 7").unwrap();
	assert_eq!(printed(&folder), "42");
	assert_eq!(run_with(&folder, &[("WARP_TIME_LIMIT", "77"), ("WARP_PHASE_TIMES", "1")]).0, "42");
	assert_eq!(cached_programs(&folder), 1);
}
