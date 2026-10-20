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
	let run = crate::common::warp_command()
		.args(["--no-ask", "run", "program.warp"])
		.current_dir(folder)
		.env(warp::run::module_cache::CACHE_VARIABLE, folder.join("cache"))
		.output()
		.unwrap();
	assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
	String::from_utf8_lossy(&run.stdout).trim().to_string()
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
