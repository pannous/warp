//! Counting a field read by name: the field lookup inside `count` needs its miss function (card fs-fs-count, a panic before)
use crate::is;

#[test]
fn test_count_of_a_field() {
	is!("s = {fs: [1,2]}; s.fs.count", 2);
	is!("s = {fs: [1,2]}; count(s.fs)", 2);
	is!("s = {name: \"ab\"}; s.name.count", 2);
}

/// `x.count()` of a parameter counts like `x.count` (it was "cannot extract a numeric value from x.(count)")
#[test]
fn a_parameter_counts_with_the_call_form() {
	is!("f(x) := x.count(); f([1, 2, 3])", 3);
	is!("f(x) := x.size(); f([1, 2])", 2);
	is!("f(x) := x.length(); f(\"abc\")", 3);
}
