// card prelude-word: Cranelift's machine code is kept per function (src/run/function_cache.rs), so a program new to the
// module cache compiles only its own functions; the runtime's and the prelude's, the same in every module, are read back
#![cfg(feature = "native")]
use std::path::Path;

/// What `warp run` prints for `code`, compiled with the folder's own cache
fn printed(folder: &Path, code: &str) -> String {
	std::fs::write(folder.join("program.warp"), code).unwrap();
	let run = crate::common::warp_command()
		.args(["--no-ask", "run", "program.warp"])
		.current_dir(folder)
		.env(warp::run::module_cache::CACHE_VARIABLE, folder.join("cache"))
		.output()
		.unwrap();
	assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
	String::from_utf8_lossy(&run.stdout).trim().to_string()
}

fn kept_functions(folder: &Path) -> usize {
	let files = std::fs::read_dir(folder.join("cache")).unwrap().filter_map(Result::ok);
	files.filter(|file| file.path().extension().is_some_and(|extension| extension == "function")).count()
}

#[test]
fn a_new_program_compiles_only_its_own_functions() {
	let folder = crate::common::scratch_directory("function_cache");
	let _ = std::fs::remove_dir_all(folder.join("cache")); // a cold cache: what the first program keeps is all its own
	std::fs::create_dir_all(folder.join("cache")).unwrap();
	assert_eq!(printed(&folder, "print to_base(255, 16)"), "ff");
	let first = kept_functions(&folder);
	assert!(first > 20, "the first program keeps the runtime's functions: {first}");
	assert_eq!(printed(&folder, "print to_base(254, 16)"), "fe");
	// besides its own functions a new module adds wasmtime's signature trampolines (about 30): their keys differ from run
	// to run under parallel compilation (notes/incremental_builds.md)
	let added = kept_functions(&folder) - first;
	assert!(added * 2 < first, "the second program adds its own few functions, not {added} of {first}");
}
