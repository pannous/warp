// A range and a value with tolerance are both closed spans: `1900 - 2000 AD` is `1950 AD ± 50` (card range-units,
// numbers::test_math::test_hyphen_units). Equal when they cover the same span in the same unit
use crate::is;

#[test]
fn a_range_equals_the_same_span_with_tolerance() {
	is!("1900 - 2000 AD == 1950 AD ± 50", true);
	is!("1900 - 2000 cm == 1950 cm ± 50", true);
	is!("1900 - 2000 cm == 1950 ± 50 cm", true);
	is!("1950 ± 50 cm == 1900 - 2000 cm", true);
	is!("1900 - 2000 AD == 1950 AD ± 40", false);
	is!("1900 - 2000 AD != 1950 AD ± 40", true);
	is!("1900 - 2000 cm == 1900 - 2000 cm", true);
	is!("1950 ± 50 == 1950 ± 50", true);
}

#[test]
fn spans_in_different_units_do_not_compare() {
	crate::common::fails_with("1900 - 2000 cm == 1950 m ± 50", "different units");
	crate::common::fails_with("1900 - 2000 cm + 1", "not supported");
}
