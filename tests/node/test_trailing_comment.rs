//! Card parse-word: a comment ending a line documents that line's item, not the next one (notes/parse_word.md)
use crate::is;

#[test]
fn comment_ending_a_line_belongs_to_its_item() {
	is!("x = data (a: 1 // one\nb: 2); (x#1).comment", "one");
	is!("x = data [a // first\nb]; (x#1).comment", "first");
	is!("x = parse(\"// lead\\na: 1 // trail\\nb: 2\"); (x#1).comment", "trail");
	is!("x = 3 // three\ny = 4\nx + y", 7);
}
