//! Card random-dispatches: random(x) by the value's type: of a list one of its elements, of a whole number n one of
//! 0…n-1, of a fraction a float from 0 up to it; a literal picks its prelude word when lowered, a variable at run time
use crate::is;

#[test]
fn random_of_a_list_is_one_of_its_elements() {
	is!("random [7,7]", 7);
	is!("xs=[7,7]; random xs", 7);
	is!("xs=[\"a\",\"a\"]; random(xs)", "a");
	is!("all([random [1 2 3] for i in 1 to 30], x => x >= 1 and x <= 3)", true);
	is!("use prelude; random_element([5])", 5);
}

#[test]
fn random_of_a_fraction_is_a_float_below_it() {
	is!("x = random 2.5; x >= 0 and x < 2.5", true);
	is!("x = random 2.5; type x", "float");
	is!("y = 0.5; x = random y; x >= 0 and x < 0.5", true);
}

#[test]
fn random_of_a_whole_number_variable_stays_whole() {
	is!("n = 6; x = random n; x >= 0 and x < 6", true);
	is!("n = 6; x = random n; type x", "int");
	is!("use prelude; random_of(1)", 0);
}

/// random never takes a truth value, so a comparison after its argument compares its result (the signature rule of
/// nest_prefix_calls): `random 6 < 6` is (random 6) < 6
#[test]
fn random_compared_takes_the_first_comparand() {
	is!("random 6 < 6", true);
	is!("random 1..4 < 4", true);
	is!("xs = [1, 2]; random xs < 3", true);
	is!("random 2.5 < 2.5 and yes", true);
	is!("random < 1", true);
}
