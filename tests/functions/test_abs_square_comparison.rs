// card abs-square: braceless prefix calls on both sides of `and` each take their own comparison
use crate::is;

#[test]
fn braceless_library_calls_compare_on_both_sides_of_and() {
	is!("use math; abs -3 < 5 and square 3 < 10", true);
	is!("use math; abs -3 < 2 and square 3 < 10", false);
}

#[test]
fn braceless_user_call_compares_beside_a_library_call() {
	is!("square(x):=x*x; abs -3 < 5 and square 3 < 10", true);
}
