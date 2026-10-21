//! Card rust-lane-literal: `[]` keeps its kind when passed or returned, as it does in a variable and through if-else: a
//! value never changes kind by a call (Interviewer 2026-10-21), and `=== ø` asks for ø itself
use crate::is;

#[cfg(feature = "native")] // the warp binary: not in the browser build
fn printed_lines(code: &str) -> Vec<String> {
	crate::common::printed(code).lines().filter(|line| !line.starts_with('»')).map(String::from).collect()
}

#[test]
#[cfg(feature = "native")]
fn a_returned_empty_list_prints_as_the_empty_list() {
	assert_eq!(printed_lines("g(): any := { return [] }\nprint g()"), ["[]"]);
	assert_eq!(printed_lines("g(k: int): any := { if k == 1 { return [] }; 3 }\nprint g(1)"), ["[]"]);
}

#[test]
fn an_empty_list_passed_or_returned_is_not_nothing_itself() {
	is!("f(n: any) := n === ø; f([])", false);
	is!("f(n: any) := n !== ø; f([])", true);
	is!("f(n: any) := n === ø; f(ø)", true);
	is!("g(): any := { return [] }; type(g())", "list");
}
