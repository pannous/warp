//! card number-converts: an exact number given to a float place converts to a float, as `x: float = 1/3` does:
//! a float field, a list of floats and a floats field
use crate::is;

#[test]
fn a_float_field_converts_a_ratio() {
	is!("class P { v: float }; P(1/3).v is float", true);
}

#[test]
fn a_floats_list_converts_its_ratios() {
	is!("xs: floats = [1/3, 2]; xs[0] is float", true);
	is!("xs: [float] = [1/3]; xs[0] is float", true);
	is!("class Q { xs: floats }; Q([1/3]).xs[0] is float", true);
}
