//! card call-name: a libm function only the C headers know (no entry in LIBM_F64_FUNCTIONS) links like the listed ones,
//! instead of compiling to its last argument (`x = exp2(3); x + 1` was 4)
use crate::is;

#[test]
fn a_libm_function_of_the_headers_links_by_itself() {
	is!("exp2(3)", 8);
	is!("x = exp2(3); x + 1", 9);
	is!("cbrt(27) + erf(0)", 3);
}

#[test]
fn a_libm_function_with_an_int_parameter_says_so() {
	crate::common::fails_with("ldexp(1, 3)", "ldexp is a libm function with a parameter other than a float");
}
