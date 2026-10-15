//! Decision foreign-signature (card standalone-math-units): `erf(x: float): float from 'm'` declares a C function with
//! its signature in warp; lib/math.warp declares libm's that way, and src/ffi reads its table from those lines
use crate::is;

#[test]
fn a_foreign_line_imports_its_function() {
	is!("erf(x: float): float from 'm'\nerf(1.0)", 0.8427007929497148);
}

#[test]
fn the_libm_table_comes_from_lib_math() {
	let functions = warp::ffi::libm_f64_functions();
	assert!(functions.contains(&("atan2", 2)), "{functions:?}");
	assert!(functions.contains(&("sin", 1)), "{functions:?}");
	assert_eq!(functions.len(), 27);
	is!("atan2(1, 1) * 4", std::f64::consts::PI);
	is!("use math; hypotenuse(3, 4)", 5.0);
}
