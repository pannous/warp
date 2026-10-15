use crate::is;

#[test]
fn ignored_parameters_still_consume_arguments() {
	is!("def constant(_):=\"hi\"; constant(1)", "hi");
	is!("def middle(_, value, _):=value; middle(1, 2, 3)", 2);
}
