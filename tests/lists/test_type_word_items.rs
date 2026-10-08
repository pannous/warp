// card variable-named: `int 4` and `int(4)` cast, but a list `[int, 4]` or a variable named like a type word is no
// cast: `int = 3; [int, 4]` is the list [3 4]
use crate::is;
use warp::warp_parser::parse;

#[test]
fn a_type_word_variable_heads_a_list() {
	is!("int = 3; count([int, 4])", 2);
	is!("int = 3; [int, 4]", parse("[3 4]"));
	is!("text = \"ab\"; [text, \"cd\"]", parse("[\"ab\" \"cd\"]"));
	is!("f(text, other) := [text, other]; f(\"ab\", \"cd\")", parse("[\"ab\" \"cd\"]"));
	is!("int(\"4\")", 4);
	is!("int 4.7", 4);
}
