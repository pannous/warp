// card pure-self-stale-cache: `use os; keep(name, value)` keeps a value for the program that computed it, across its
// runs; another program (an older compiler.wasm of the same lib/ under `warp pure`) reads none of it
#![cfg(feature = "native")]

const KEEPING: &str = "use os\nwas = kept(\"owner\")\nkeep(\"owner\", \"first\")\nwas\n";
const READING: &str = "use os\nkept(\"owner\")\n";

/// What `warp <file>` prints for `code`, run with the folder's own module cache
fn printed(folder: &std::path::Path, file: &str, code: &str) -> String {
	std::fs::write(folder.join(file), code).unwrap();
	let run = crate::common::warp_command()
		.args(["--no-ask", file])
		.current_dir(folder)
		.env(warp::run::module_cache::CACHE_VARIABLE, folder.join("cache"))
		.env("WARP_HINTS", "0")
		.output()
		.unwrap();
	String::from_utf8_lossy(&run.stdout).trim().to_string() + &String::from_utf8_lossy(&run.stderr)
}

#[test]
fn a_kept_value_belongs_to_the_program_that_kept_it() {
	let folder = crate::common::scratch_directory("kept_values");
	let _ = std::fs::remove_dir_all(folder.join("cache"));
	std::fs::create_dir_all(folder.join("cache")).unwrap();
	assert_eq!(printed(&folder, "first.warp", KEEPING), "");
	assert_eq!(printed(&folder, "first.warp", KEEPING), "\"first\"");
	assert_eq!(printed(&folder, "second.warp", READING), "");
}
