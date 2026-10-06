//! Structural patterns in `switch`/`match` (wiki/pattern-matching.md, row T5): a list pattern matches a list of its length;
//! literal parts compare, names bind the element, `_` matches anything, nested lists and pairs match inside.
use crate::common::fails_with;
use crate::is;

const GREET: &str = "greet(xs) := switch xs { [\"\", middle, \"\"]: middle + \"!\"  [\"foo\", x]: x  _: \"never mind\" }; ";

#[test]
fn test_list_patterns_bind_names() {
	is!(&format!("{GREET}greet([\"\", \"hi\", \"\"])"), "hi!");
	is!(&format!("{GREET}greet([\"foo\", \"bar\"])"), "bar");
	is!(&format!("{GREET}greet([\"a\", \"b\", \"c\"])"), "never mind");
	is!(&format!("{GREET}greet([\"\", \"hi\"])"), "never mind"); // the length must match
}

#[test]
fn test_wildcards_and_numbers() {
	is!("xs=[1, 2, 3]; switch xs { [1, _, z]: z * 10  default: 0 }", 30);
	is!("xs=[2, 2, 3]; switch xs { [1, _, z]: z * 10  default: 0 }", 0);
	is!("xs=[]; switch xs { []: 1  [_]: 2  _: 3 }", 1);
	is!("xs=[7]; switch xs { []: 1  [_]: 2  _: 3 }", 2);
}

#[test]
fn test_nested_lists_and_pairs() {
	is!("xs=[1, [2, [3]]]; switch xs { [outer, [middle, [inner]]]: outer + middle + inner  _: 0 }", 6);
	is!("xs=[1, 2]; switch xs { [outer, [middle, [inner]]]: outer + middle + inner  _: 0 }", 0);
	is!("p = [a: 1, b: 2]; switch p { [a: x, b: y]: x + y  _: 0 }", 3);
}

#[test]
fn test_a_subject_that_is_no_list_does_not_match() {
	is!("x=5; switch x { [a]: a  _: 0 }", 0);
}

#[test]
fn test_no_match_without_default_is_loud() {
	fails_with("xs=[1, 2]; switch xs { [a]: a }", "no case for xs");
}
