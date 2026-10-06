// `count x in y`: how often x occurs in y (a character, an item, or a text in a text); `count bytes in t` counts a unit
use crate::is;

#[test]
fn count_occurrences() {
	is!("count 'a' in \"banana\"", 3);
	is!("count \"an\" in \"banana\"", 2);
	is!("s=\"banana\"; count 'a' in s", 3);
	is!("count 2 in [1 2 2 3]", 2);
	is!("count 5 in [1 2 3]", 0);
	is!("xs=[\"a\" \"b\" \"a\"]; count \"a\" in xs", 2);
	is!("count bytes in \"äb\"", 3);
}
