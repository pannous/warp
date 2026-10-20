//! card dir-lists: `dir` of a list or text lists the builtin methods too (count sum map sort …, members.rs
//! LIST_MEMBERS / TEXT_MEMBERS, the names member completion offers), after the words of the module named after the type
use crate::is;

#[test]
fn dir_of_a_list_lists_its_builtin_methods() {
	is!("(\"count\" in dir([1 2])) > 0", true);
	is!("\"add\" in dir([1 2])", true);
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

#[test]
fn methods_list_what_dir_lists() {
	is!("xs = [1]\ncount(xs.methods) == count(dir(xs))", true);
	is!("(\"Cursor\" in dir(text)) > 0", false);
}

#[test]
#[ignore = "fixnow: fixnow-builtin-function-add: left: [1] right: [1 2] (since ocean 65c3fff1a0)"]
fn every_builtin_method_is_a_function_value() {
	is!("f = function count; f(\"abc\")", 3);
	is!("f = function filter; f([1 2 3], x => x > 1)", warp::ints(vec![2, 3]));
	is!("f = function add; xs = [1]; f(xs, 2); xs", warp::ints(vec![1, 2]));
	is!("f = function insert; f([1 2], 5, 1)", warp::ints(vec![1, 5, 2]));
	is!("count([function upper, function min])", 2); // min is the function, no minute
}
