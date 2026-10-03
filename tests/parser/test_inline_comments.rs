use warp::*;

#[test]
fn test_block_comment_between_operands() {
	is!("1 /* inline */ + 1", 2);
	is!("1 + /* inline */ 1", 2);
	is!("1 /* a */ + /* b */ 1 /* c */", 2);
}

#[test]
fn test_hash_comment_after_a_statement_keeps_the_newline_separator() {
	is!("x=3 # note\nx+1", 4);
}

#[test]
fn test_hash_comment_ends_the_expression_not_the_program() {
	is!("x=3 # to silence python warnings;)\n x*2", 6);
}

#[test]
fn test_hash_without_space_is_still_the_index_operator() {
	is!("x=[1 2 3];x#2", 2);
}

#[test]
fn test_block_comment_between_name_and_assignment() {
	is!("y/* yeah! */=5;y", 5);
}
