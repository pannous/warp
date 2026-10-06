use crate::is;

#[test]
fn test_statement_sequence_with_grouped_list_in_the_middle() {
	is!("'hello';(1 2 3 4);10", 10);
}
