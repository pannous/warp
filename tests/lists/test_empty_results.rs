//! Card empty-results: a list operation that leaves no items gives the empty list `[]`, as `[]` itself is (card
//! run-time-empty): an empty slice, a filter keeping nothing, a sort or reverse of nothing, the pop of the last item
use crate::is;

#[cfg(feature = "native")] // the warp binary: not in the browser build
fn printed_lines(code: &str) -> Vec<String> {
	crate::common::printed(code).lines().filter(|line| !line.starts_with('»')).map(String::from).collect()
}

#[test]
#[cfg(feature = "native")]
fn an_empty_result_prints_as_the_empty_list() {
	assert_eq!(printed_lines("xs = [1, 2]\nprint xs[0..<0]"), ["[]"]);
	assert_eq!(printed_lines("xs = [1, 2]\nprint xs.filter(x => x > 5)"), ["[]"]);
	assert_eq!(printed_lines("xs = [1]\nxs.pop()\nprint xs"), ["[]"]);
	assert_eq!(printed_lines("out = []\nfor x in [1, 2] { if x > 5 { out += [x] } }\nprint out"), ["[]"]);
}

#[test]
fn an_empty_result_is_a_list() {
	is!("xs = [1, 2]; type(xs[0..<0])", "list");
	is!("xs = [1]; xs.pop(); type(xs)", "list");
	is!("xs = [1, 2]; ys = xs.filter(x => x > 5); ys + \"a\"", warp::parse("[\"a\"]"));
	is!("xs = [1, 2]; xs[0..<0] == ø", false); // decision empty-is-not-missing
}
