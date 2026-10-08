// card type-static: type(x) of a variable whose static type was guessed wrong: a decimal held in a variable is rational,
// ø is empty (canonical word, warp-e9), an instance of a declared class names its class
use crate::is;

#[test]
fn the_type_of_a_variable_is_its_value_type() {
	is!("x = 1.5; type(x)", "rational");
	is!("type(ø)", "empty");
	is!("x = ø; type(x)", "empty");
	is!("class P{x:int}; p = P(1); type(p)", "P");
	is!("x = 3; type(x)", "int");
}
