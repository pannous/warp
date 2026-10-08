// `print chars in "hello"` prints each character, like `for chars in "hello": print it` (wiki/in.md); a variable of
// that name keeps `in` the membership test
use crate::is;

#[test]
fn print_name_in_a_text_walks_it() {
	is!("print chars in \"hello\"", warp::Node::Empty); // P213: a loop ending in print gives ø
	is!("c = \"e\"; print c in \"hello\"", warp::Node::Empty); // print gives nothing (issue #18)
}
