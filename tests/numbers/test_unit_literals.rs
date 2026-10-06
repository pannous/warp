// A unit written after an amount with a space, by its long name too: `3010 meters` is 3010 m, and after an expression
// it belongs to the expression's last amount, so it binds tighter than the comparison (wiki/number.md
// `3km+10m = 3010 meters`)
use crate::is;

#[test]
fn a_unit_after_a_comparison_belongs_to_its_amount() {
	is!("3km+10m == 3010 meters", true);
	is!("3km+10m == 3011 meters", false);
	is!("3km == 3 kilometers", true);
}

#[test]
fn a_variable_named_like_a_unit_stays_a_variable() {
	is!("meters = 5; meters + 1", 6);
}
