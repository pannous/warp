//! card cross-module-overload: a path written without quotes is a Path where a word has a method for one
//! (lib/shell.warp): `sort ./names.txt` sorts the file's lines, `sort [3 1 2]` the list; the same for unique, find, size
#![cfg(feature = "native")]
use super::test_shell_words::script_in;

const NAMES: &str = "write(\"n.txt\", \"pear\\napple\\npear\\n\")\n";

#[test]
fn a_path_takes_the_file_word_a_list_the_list_word() {
	let code = format!("{NAMES}print sort ./n.txt\nprint sort [3 1 2]\nprint unique ./n.txt\nprint unique [1 1 2]\nprint size ./n.txt\nprint [4 5].size");
	let expected = "[\"apple\" \"pear\" \"pear\"]\n[1 2 3]\n[\"pear\" \"apple\"]\n[1 2]\n16\n2\n";
	assert_eq!(script_in("overload_words", &code), expected);
}

#[test]
fn a_variable_holding_a_path_is_a_path() {
	let code = format!("{NAMES}notes = ./n.txt\nprint sort notes");
	assert_eq!(script_in("overload_variable", &code), "[\"apple\" \"pear\" \"pear\"]\n");
}

#[test]
fn find_searches_a_folder_or_a_list() {
	let code = "mkdir ./d\ntouch ./d/a.txt\ntouch ./d/b.md\nprint find(./d, \"*.txt\")\nprint find ./d \"*.md\"\nprint find([1 2 3], x => x > 1)\nprint [1 2 3].find(x => x > 1)";
	let expected = "[\"./d/a.txt\"]\n[\"./d/b.md\"]\n2\n2\n";
	assert_eq!(script_in("overload_find", code), expected);
}

#[test]
fn the_current_folder_alone_is_a_path() {
	let code = "touch ./a.txt\nprint ls ./\nxs = [2 4]\nprint xs ./ 2";
	assert_eq!(script_in("overload_dot_slash", code), "[\"a.txt\"]\n[1 2]\n");
}
