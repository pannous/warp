// card unit-glued, decision litre-suffix: a unit after a number, named like a variable of the program, is an error
// without `use units`; with it the unit (decision unit-after-number), and a written `*` multiplies by the variable
use crate::common::fails_with;
use crate::is;

#[test]
fn a_unit_named_like_a_variable_needs_use_units() {
	fails_with("m = 5; 3m", "m is both a unit and a variable here; add `use units` for the unit, or write 3*m");
	fails_with("s = 2; 0.5s", "write 0.5*s to multiply");
	is!("use units; m = 5; str(3m)", "3m");
	is!("use units\ns = 2\nstr(0.5s)", "0.5s");
}

#[test]
fn a_written_product_or_another_name_needs_nothing() {
	is!("m = 5; 3*m", 15);
	is!("m = 5; m * 3", 15);
	is!("x = 5; 3x", 15);
	is!("str(3m + 2m)", "5m");
}
