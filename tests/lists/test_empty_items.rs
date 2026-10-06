// A ø item of a list stays: read back (it was left out: [1 'a' 'z'] for four items) and in the list's text
use crate::is;
use warp::*;

#[test]
fn a_list_keeps_its_empty_items() {
	is!("[1, \"a\", ø, \"z\"]", list(vec![int(1), text("a"), Node::Empty, text("z")]));
	is!("[ø]", list(vec![Node::Empty]));
}

#[test]
fn an_empty_item_reads_as_empty_in_the_text_of_a_list() {
	is!("xs = [1, ø]; str(xs)", "[1 ø]");
}
