//! card floats-add-exact: an exact number added to a declared list of floats converts to a float, as one in its
//! initial items does (card number-converts): `xs.add(1/3)`, `xs + [1/3]`, a variable's item, a floats field.
//! `xs[1] is float` answers from the declared element type, so the tests read the item's text.
use crate::is;

const FLOAT_THIRD: &str = "0.3333333333333333";

#[test]
fn an_added_ratio_converts() {
	is!("xs: floats = [1/3]; xs.add(1/3); \"\" + xs[1]", FLOAT_THIRD);
	is!("xs: [float] = [0.5 as float]; xs.add(1/3); \"\" + xs[1]", FLOAT_THIRD);
	is!("xs: floats = [1/3]; y = 1/3; xs.add(y); \"\" + xs[1]", FLOAT_THIRD);
}

#[test]
fn a_concatenated_ratio_converts() {
	is!("xs: floats = [1/3]; xs = xs + [1/3]; \"\" + xs[1]", FLOAT_THIRD);
}

#[test]
fn a_floats_field_converts_an_added_ratio() {
	is!("class Q { xs: floats }; q = Q([1/3]); q.xs.add(1/3); \"\" + q.xs[1]", FLOAT_THIRD);
}

#[test]
fn a_numbers_list_keeps_an_added_ratio() {
	is!("xs: numbers = [1/3]; xs.add(1/3); \"\" + xs[1]", "1/3");
}
