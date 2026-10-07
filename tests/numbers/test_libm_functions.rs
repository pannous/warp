//! libm functions beyond the first fifteen: each linked, none compiles to its last argument (hypot(3, 4) was 4)
use crate::is;

#[test]
#[allow(clippy::approx_constant, reason = "the digits libm returns, written out as the program prints them")]
fn trigonometry_and_logarithms_from_libm() {
	is!("hypot(3, 4)", 5);
	is!("atan2(1, 1) * 4", 3.141592653589793);
	is!("asin(1) * 2", 3.141592653589793);
	is!("acos(1) + atan(0) + sinh(0) + tanh(0)", 0);
	is!("cosh(0)", 1);
	is!("log2(8)", 3);
	is!("trunc(2.7)", 2);
	is!("x = hypot(6, 8); x + 1", 11);
}

#[test]
fn test_cube_root_of_a_runtime_value() {
	is!("f(x) := ∛x; f(8)", 2.0);
	is!("f(x) := ∛x; f(-27)", -3.0);
	is!("f(x) := cbrt x; f(64) + 1", 5.0);
}
