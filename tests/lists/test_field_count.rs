//! Counting a field read by name: the field lookup inside `count` needs its miss function (card fs-fs-count, a panic before)
use crate::is;

#[test]
fn test_count_of_a_field() {
	is!("s = {fs: [1,2]}; s.fs.count", 2);
	is!("s = {fs: [1,2]}; count(s.fs)", 2);
	is!("s = {name: \"ab\"}; s.name.count", 2);
}
