// card browser-twin: the scripting words of `use script` (decision use-script) in the browser too (web/playground/
// host-files.js os and file): glob of the written and served files, cwd the folder relative paths start from, no input;
// command runs a shell natively only, in the browser the error says so
use crate::is;
use warp::warp_parser::parse;

const FOLDER: &str = "scratch/test_script_words";

#[test]
fn glob_finds_the_written_files() {
	let folder = format!("{FOLDER}/glob"); // its own folder: the other tests write files of their own beside FOLDER
	let _ = std::fs::create_dir_all(&folder);
	let writes = ["b.txt", "a.txt", "c.md"].map(|file| format!("write(\"{folder}/{file}\", \"x\")"));
	is!(
		&format!("use script; {}; glob(\"{folder}/*.txt\")", writes.join("; ")),
		parse(&format!("[\"{folder}/a.txt\" \"{folder}/b.txt\"]"))
	);
}

#[test]
fn a_path_from_cwd_is_found() {
	let _ = std::fs::create_dir_all(FOLDER);
	is!(&format!("use script; write(cwd + \"/{FOLDER}/here.txt\", \"x\"); exists(\"{FOLDER}/here.txt\")"), true);
}

#[cfg(not(feature = "native"))]
#[test]
fn a_page_has_no_input_and_no_shell() {
	is!("use script; stdin()", "");
	is!("use script; input() == ø", true);
	let refused = warp::wasm_emitter::eval("use script; command(\"echo hi\")").serialize();
	assert!(refused.contains("not in the browser"), "{refused}");
}
