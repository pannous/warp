// `print chars in "hello"` prints each character, like `for chars in "hello": print it` (wiki/in.md); a variable of
// that name keeps `in` the membership test
use warp::*;

#[test]
fn print_name_in_a_text_walks_it() {
	is!("print chars in \"hello\"", 5);
	is!("c = \"e\"; print c in \"hello\"", 2);
}
