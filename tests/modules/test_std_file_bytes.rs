//! card path-list: a file's bytes as a list of ints, written and read back (a warp-written emitter writes a .wasm)
use crate::common;
use crate::is;
use warp::warp_parser::parse;

const FOLDER: &str = "scratch/test_std_file_bytes";

#[test]
fn write_bytes_and_read_bytes_need_no_use() {
	let _ = std::fs::create_dir_all(FOLDER);
	let path = format!("{FOLDER}/header.wasm");
	is!(
		&format!("write_bytes(\"{path}\", [0 97 115 109 1 0 0 0]); read_bytes(\"{path}\")"),
		parse("[0 97 115 109 1 0 0 0]")
	);
	is!(&format!("write_bytes(\"{path}\", [128, 255]); read_bytes(\"{path}\")"), parse("[128 255]"));
}

#[test]
fn bytes_and_texts_are_the_same_file() {
	let _ = std::fs::create_dir_all(FOLDER);
	let path = format!("{FOLDER}/hi.txt");
	is!(&format!("write_bytes(\"{path}\", [104, 105]); read(\"{path}\")"), "hi");
	is!(&format!("write(\"{path}\", \"hey\"); read_bytes(\"{path}\")"), parse("[104 101 121]"));
}

#[test]
fn a_value_that_is_no_byte_is_refused() {
	let _ = std::fs::create_dir_all(FOLDER);
	let path = format!("{FOLDER}/refused.bin");
	common::fails_with(&format!("write_bytes(\"{path}\", [1, 256])"), "256 is no byte");
	common::fails_with(&format!("write_bytes(\"{path}\", [-1])"), "-1 is no byte");
}

#[test]
fn writing_creates_missing_folders() {
	let folder = format!("{FOLDER}/created");
	let _ = std::fs::remove_dir_all(&folder);
	is!(&format!("write_bytes(\"{folder}/a/b.bin\", [1, 2]); read_bytes(\"{folder}/a/b.bin\")"), parse("[1 2]"));
	is!(&format!("write(\"{folder}/c/d.txt\", \"hi\"); read(\"{folder}/c/d.txt\")"), "hi");
}
