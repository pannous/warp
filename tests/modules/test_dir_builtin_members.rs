//! card dir-lists: `dir` of a list or text lists the builtin methods too (count sum map sort …, members.rs
//! LIST_MEMBERS / TEXT_MEMBERS, the names member completion offers), after the words of the module named after the type
use crate::is;

#[test]
fn dir_of_a_list_lists_its_builtin_methods() {
	is!("(\"count\" in dir([1 2])) > 0", true);
	is!("xs = [3 4]; (\"sort\" in dir(xs)) > 0", true);
	is!("(\"sum\" in dir(list)) > 0", true);
	is!("(\"unique\" in dir([1 2])) > 0", true);
	is!("f(v) := dir(v); (\"map\" in f([1])) > 0", true);
}

#[test]
fn dir_of_a_text_lists_its_builtin_methods() {
	is!("(\"upper\" in dir(\"hi\")) > 0", true);
	is!("(\"trim\" in dir(text)) > 0", true);
	is!("dir(text)#1", "repeat");
}
