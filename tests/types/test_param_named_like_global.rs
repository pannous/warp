// card param-named-like-global: a function's parameter is its own value, whatever a program variable of the same name
// holds (an instance of a class, a list)
use crate::is;

#[test]
fn a_parameter_named_like_an_instance_variable_compares_as_its_own_value() {
	is!("class V {n:int; equals(o:V) := n == o.n}\nf(a, b) := if a == b then a + \"²\" else a + \"·\" + b\na = V(1)\nf(\"m\", \"m\")", "m²");
	is!("unit_product(a, b) := if a == b then a + \"²\" else a + \"·\" + b\na = quantity(2, \"m\") * quantity(3, \"m\")\nunit_product(\"m\", \"s\")", "m·s");
}

#[test]
fn a_parameter_named_like_a_list_variable_takes_one_value() {
	is!("f(a, b) := a + \"²\"\na = [1]\nf(\"m\", \"m\")", "m²");
	is!("f(a) := a + \"²\"\na = [1]\nf(\"m\")", "m²");
}
