//! card number-param-int: a parameter typed `number` takes any number, a float too, also when the argument is held
//! as a Node (`any`)
use crate::is;

#[test]
fn a_number_parameter_takes_a_float() {
	is!("f(n: number) := n is float; f(2.5 as float)", true);
	is!("f(n: number) := n; g(n: any) := f(n); g(2.5 as float)", 2.5);
}

#[test]
fn a_number_parameter_given_any_keeps_the_kind() {
	let program = "number_kind(n: number): text := if n is int then \"int\" else \"other\"
		kind_of(n: any): text := if n is number then number_kind(n) else \"no number\"";
	is!(&format!("{program}\nkind_of(2.5 as float)"), "other");
	is!(&format!("{program}\nkind_of(3)"), "int");
	is!(&format!("{program}\nkind_of(\"a\")"), "no number");
}
