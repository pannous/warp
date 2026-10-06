// A one-character text parses as a codepoint; assigned to a variable declared string/text it is that one-character text
use crate::is;

#[test]
fn a_declared_text_takes_a_one_character_text() {
	is!("string x = \"a\"; x", "a");
	is!("x:string = \"a\"; x + \"b\"", "ab");
	is!("text x = \"a\"; x", "a");
	is!("string x = \"ab\"; x = \"c\"; x + \"d\"", "cd");
}
