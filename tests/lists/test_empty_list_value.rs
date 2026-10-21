//! Card run-time-empty: `[]` stays the empty list at run time: it prints as `[]`, its type is list, and wherever it
//! goes (returned, passed, held in a list) `+=` appends to it; it still equals ø
use crate::is;
use warp::parse;

#[cfg(feature = "native")] // the warp binary: not in the browser build
fn printed_lines(code: &str) -> Vec<String> {
	crate::common::printed(code).lines().filter(|line| !line.starts_with('»')).map(String::from).collect()
}

#[test]
#[cfg(feature = "native")]
fn an_empty_list_prints_as_written() {
	assert_eq!(printed_lines("print []"), ["[]"]);
	assert_eq!(printed_lines("print [[], []]"), ["[[] []]"]);
	assert_eq!(printed_lines("show(xs) := print xs\nshow([])"), ["[]"]);
	assert_eq!(printed_lines("xs = []\nprint xs"), ["[]"]);
}

#[test]
fn an_empty_list_is_a_list() {
	is!("type([])", "list");
	is!("[] == ø", true);
}

#[test]
fn a_returned_empty_list_takes_items() {
	is!("fresh() := []\nt = fresh()\nt += \"a\"\nt", parse("[\"a\"]"));
	is!("fresh() := []\nt = fresh()\nt += 1\ncount t", 1);
}
