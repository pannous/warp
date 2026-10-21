//! `warp a.wasm -- x y` (card wasm-args): the words after `--` are the program's arguments (`use os; args`), for a .warp
//! file and its compiled .wasm alike; `warp a.wasm x y` takes x y as arguments too, with a note naming the `--` form
use std::path::Path;

const PROGRAM: &str = "use os; print(count(args)); print(args#2)";

fn printed(folder: &Path, arguments: &[&str]) -> String {
	let run = crate::common::warp_command().current_dir(folder).args(arguments).env("WARP_NO_WINDOW", "1").output();
	let run = run.expect("warp runs");
	assert!(run.status.success(), "{arguments:?}: {}", String::from_utf8_lossy(&run.stderr));
	String::from_utf8_lossy(&run.stdout).into_owned()
}

#[test]
fn the_words_after_two_dashes_are_the_programs_arguments() {
	let folder = crate::common::scratch_directory("program_arguments");
	std::fs::create_dir_all(&folder).unwrap();
	std::fs::write(folder.join("a.warp"), PROGRAM).unwrap();
	printed(&folder, &["compile", "--wasm", "a.warp"]);
	for arguments in [&["a.warp", "--", "x", "y"][..], &["a.wasm", "--", "x", "y"], &["a.wasm", "x", "y"]] {
		assert_eq!(printed(&folder, arguments), "2\ny\n", "{arguments:?}");
	}
}
