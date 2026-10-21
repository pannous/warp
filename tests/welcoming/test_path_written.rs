//! card path-written: a path written without quotes stays a Path through the program's functions: a parameter every
//! call passes such a path, and a call of a function giving only such paths (warp_parser WrittenPaths)
#![cfg(feature = "native")]
use super::test_shell_words::script_in;

const NAMES: &str = "write(\"n.txt\", \"pear\\napple\\npear\\n\")\n";
const SORTED: &str = "[\"apple\" \"pear\" \"pear\"]\n";

#[test]
fn a_parameter_passed_a_path_is_a_path() {
	let code = format!("{NAMES}f(p) := sort p\nprint f(./n.txt)");
	assert_eq!(script_in("path_parameter", &code), SORTED);
}

#[test]
fn a_function_giving_a_path_gives_a_path() {
	let code = format!("{NAMES}g() := ./n.txt\nprint sort g()");
	assert_eq!(script_in("path_result", &code), SORTED);
}

#[test]
fn a_path_flows_through_several_functions() {
	let code = format!("{NAMES}notes = ./n.txt\nh(q) := sort q\nf(p) := h(p)\nprint f(notes)");
	assert_eq!(script_in("path_forwarded", &code), SORTED);
}
