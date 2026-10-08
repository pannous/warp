// card banana-count: `x.count(y)` and `count(x, y)` are `count y in x`, as Python's "banana".count("a"); a program's own
// count function keeps its calls
use crate::is;

#[test]
fn count_as_a_method_or_a_call_of_two() {
	is!("\"banana\".count('a')", 3);
	is!("\"banana\".count(\"an\")", 2);
	is!("[1, 2, 1].count(1)", 2);
	is!("count(\"banana\", 'a')", 3);
	is!("s = \"banana\"; s.count('n')", 2);
}

#[test]
fn a_defined_count_keeps_its_calls() {
	is!("def count(a, b){a + b}; count(3, 4)", 7);
	is!("def count(a, rest...){#rest}; count(7, 8, 9)", 2);
}
