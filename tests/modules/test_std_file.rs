//! The standard library modules file and os (notes/stdlib.md section 7): their words call the adapter std_io, natively
//! the file system and environment, in the browser host.js's in-memory files (read sees what write wrote) and no
//! environment
use crate::is;
use warp::warp_parser::parse;

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

/// P171 (user, 2026-10-07): write and exists are prelude words, no `use file`; a program's own word of that name wins
#[test]
fn write_and_exists_need_no_use() {
	let _ = std::fs::create_dir_all(FOLDER);
	let path = format!("{FOLDER}/prelude.txt");
	is!(&format!("write(\"{path}\", \"x\"); exists(\"{path}\")"), true);
	is!(&format!("exists(\"{FOLDER}/no such file\")"), false);
	is!("write(x, y) := x + y; write(1, 2)", 3);
}

/// P183 (user, 2026-10-07): a file URL loads the file module; a file's append is written qualified, file.append
#[test]
fn a_file_url_loads_the_module_and_append_is_qualified() {
	let folder = format!("{FOLDER}/url");
	let _ = std::fs::create_dir_all(&folder);
	is!(&format!("home = \"file://{folder}\"; write(\"{folder}/a.txt\", \"1\"); list_files(\"{folder}\")"), parse("[\"a.txt\"]"));
	let path = format!("{FOLDER}/appended.txt");
	is!(&format!("use file; write(\"{path}\", \"a\"); file.append(\"{path}\", \"b\"); read(\"{path}\")"), "ab");
	is!("use file; xs = [1]; xs.append(2); xs", parse("[1 2]"));
}
