// card pure-typed-params: parameters typed by a phrase or a union and an optional char result compile in every lane
use crate::is;

#[test]
fn a_parameter_typed_list_of_int_takes_the_list() {
	is!("f(xs: list of int) := #xs; f([1, 2, 3])", 3);
}

#[test]
fn a_parameter_of_a_union_type_takes_either() {
	is!("g(x: text|list) := #x; g(\"ab\") + g([1])", 3);
}

#[test]
fn an_optional_char_result() {
	is!("h(t: text): char? := t#1; h(\"xy\")", 'x');
}
