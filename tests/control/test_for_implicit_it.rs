// wiki/for.md: `for 1..10 : print it` names no loop variable, the items are `it`; a colon body runs to the end of the line
use warp::is;

#[test]
fn a_for_loop_without_a_variable_walks_it() {
	is!("s=0; for 1..4 : s+=it; s", 6);
	is!("s=0; for 1...4 : s+=it; s", 10);
	is!("s=0; for 1 to 4 : s+=it; s", 10);
	is!("s=0; for 1..4 { s+=it }; s", 6);
	is!("s=0; xs=[5 6]; for xs do s+=it; s", 11);
}

#[test]
fn a_colon_body_is_the_rest_of_the_line() {
	is!("x=2\nfor i in 1..3 : print i\nx", 2);
	is!("s=0\nfor 1..3 : s += it * 10\ns", 30);
	is!("t=\"\"\nfor i in 1..3 : t = t + str(i)\nt", "12");
}
