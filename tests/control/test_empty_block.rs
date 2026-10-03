use warp::wasp_parser::parse;
use warp::{Bracket, Node, Separator};

#[test]
fn an_empty_block_inside_a_statement_is_a_value() {
	assert_eq!(parse("x i {}\ny w {}").size(), 2);
	assert_eq!(parse("package p;\ninterface i {}\nworld w {}").size(), 3);
	assert_eq!(parse("a {}")[1], Node::List(Vec::new(), Bracket::Curly, Separator::None));
}

#[test]
fn a_program_that_is_only_an_empty_block_is_empty() {
	assert_eq!(parse("{}"), Node::Empty);
}
