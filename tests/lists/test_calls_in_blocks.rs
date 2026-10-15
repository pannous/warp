use crate::is;

#[test]
fn calls_are_lowered_inside_single_statement_blocks() {
	is!("if yes {sum [1 2]}", 3);
	is!("if yes {sum 1+1 2+2}", 6);
}
