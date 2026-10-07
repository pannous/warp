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

// card parser-if: a long name joins its amount inside a condition too, before `:` or a block
#[test]
fn a_long_unit_name_belongs_to_its_amount_in_a_condition() {
	is!("x = 2 km; if x > 1500 meters : \"far\" else \"near\"", "far");
	is!("x = 2 km; if x > 3 kilometers { \"far\" } else { \"near\" }", "near");
	is!("if 2 kilograms < 3000 grams : 1 else 2", 1);
}

#[test]
fn a_variable_named_like_a_unit_stays_a_variable() {
	is!("meters = 5; meters + 1", 6);
}
