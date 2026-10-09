//! P231: print rounds a quantity's fraction to two decimals, `6.17km`; str() and the result keep the exact `(37/6)km`;
//! plain numbers keep `7/3`
use warp::wasm_emitter::eval;
#[cfg(feature = "native")]
use crate::common::printed;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn str_and_the_result_keep_the_exact_quantity() {
	assert_eq!(shown("x = 37 km / 6\nx"), "(37/6)km");
	assert_eq!(shown("x = 37 km / 6\nstr(x)"), "\"(37/6)km\"");
	assert_eq!(shown("x = 37 km / 6\n\"x: \" + x"), "\"x: (37/6)km\"");
}

#[cfg(feature = "native")]
#[test]
fn print_rounds_a_quantity_to_two_decimals() {
	assert_eq!(printed("print 37 km / 6").trim(), "6.17km");
	assert_eq!(printed("x = 1 m / 3\nprint x as cm").trim(), "33.33cm");
	assert_eq!(printed("x = 3 km / 2\nprint x").trim(), "1.5km");
	assert_eq!(printed("xs = [37 km / 6, 2 km]\nprint xs").trim(), "[6.17km 2km]");
}

#[cfg(feature = "native")]
#[test]
fn print_keeps_a_plain_fraction() {
	assert_eq!(printed("print 7/3").trim(), "7/3");
}
