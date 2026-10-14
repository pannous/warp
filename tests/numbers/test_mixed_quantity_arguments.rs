// card mixed-arguments: a function called with a quantity and with a plain number serves both, each call by its own
// argument's class (class_methods.rs specialized_calls)
use crate::is;

#[test]
fn a_function_takes_a_quantity_and_a_plain_number() {
	is!("f(x) := x * 2; a = f(5 m ± 1 cm); b = f(3); str(a) + \" \" + str(b)", "10.000 ± 0.020m 6");
	is!("f(x) := x * 2 + x; str(f(quantity(\"1 m\"))) + \" \" + str(f(3))", "3m 9");
	is!("f(x) := x * 2; str(f(quantity(\"5 m\") / 5))", "2m");
	is!("f(x) := x * 2; [f(3), f(4)]", warp::ints(vec![6, 8]));
}

#[test]
fn a_plain_number_where_a_unit_is_needed_stays_a_dimension_error() {
	crate::common::fails_with("f(x) := x + 1 m; str(f(quantity(\"5 m\"))) + str(f(3))", "DimensionError");
}
