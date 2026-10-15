//! Card variable-named-like (decision unit-definitions): unit symbols bind after a number, the program's names
//! everywhere else; a variable or parameter named like a unit no longer turns `0.5s` or `3 km/h` into its value
use crate::is;

#[test]
fn a_unit_after_a_number_is_the_unit_though_a_variable_has_its_name() {
	is!("s = 3\nstr(0.5s)", "0.5s");
	is!("s = 3\nstr(2 * 0.5s)", "1s");
	is!("s = 3\ns * 2", 6);
	is!("h = 2 inch\nv = 3 km/h\nstr(v)", "3km/h");
	is!("s = 2\nstr(9.8 m/s²)", "9.8m/s²");
}

#[test]
fn a_parameter_named_like_a_unit_leaves_the_unit_to_the_rest() {
	is!("f(s) := s + 1\nf(1) + (0.5s as ms) / 1ms", 502);
}

#[test]
fn an_alias_and_a_declared_unit_type_see_through_the_variable() {
	is!("s = 3\nstr(440Hz)", "440/s");
	is!("h = 2 m\nv: km/h = 3 km/h\nstr(v)", "3km/h");
}
