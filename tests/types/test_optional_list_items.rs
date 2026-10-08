// card type-list: a list of values and ø is a list of optionals (P179: optional and sum types share one mechanism),
// its ø items have the type empty
use crate::is;

#[test]
fn a_list_with_ø_items_is_a_list_of_optionals() {
	is!("xs = [1, ø]; type(xs)", "list of int?");
	is!("xs = [1, ø]; type(xs#2)", "empty");
	is!("xs = [1, ø]; type(xs#1)", "int");
	is!("xs = [1, ø, 3]; xs#3 + 1", 4);
	is!("xs = [ø, \"a\"]; type(xs)", "list of codepoint?");
	is!("xs = [1, 2]; type(xs)", "list of int");
}
