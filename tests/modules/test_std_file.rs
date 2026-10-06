//! The standard library modules file and os (notes/stdlib.md section 7): their words call the adapter std_io, natively
//! the file system and environment, in the browser host.js's in-memory files (read sees what write wrote) and no
//! environment
use crate::is;
use warp::wasp_parser::parse;

const FOLDER: &str = "scratch/test_std_file";

#[test]
fn use_file_writes_appends_and_reads_back() {
	let _ = std::fs::create_dir_all(FOLDER);
	let path = format!("{FOLDER}/greeting.txt");
	is!(&format!("use file; write(\"{path}\", \"hello\"); read(\"{path}\")"), "hello");
	is!(&format!("use file; write(\"{path}\", \"hello\"); append_file(\"{path}\", \"!\"); read(\"{path}\")"), "hello!");
	is!(&format!("use file; write(\"{path}\", \"ab\ncd\"); lines(\"{path}\")"), parse("[\"ab\" \"cd\"]"));
	is!(&format!("use file; write(\"{path}\", \"x\"); exists(\"{path}\")"), true);
	is!(&format!("use file; exists(\"{FOLDER}/no such file\")"), false);
}

#[test]
fn use_file_lists_a_folder() {
	let folder = format!("{FOLDER}/listed");
	let _ = std::fs::create_dir_all(&folder);
	is!(&format!("use file; write(\"{folder}/b.txt\", \"2\"); write(\"{folder}/a.txt\", \"1\"); list_files(\"{folder}\")"), parse("[\"a.txt\" \"b.txt\"]"));
}

#[test]
fn use_os_reads_the_environment() {
	is!("use os; env(\"WARP_TEST_NO_SUCH_VARIABLE\")", parse("ø"));
}
