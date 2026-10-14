// glibc's math.h declares libm through bits/mathcalls.h macros (`__MATHCALL (sqrt,, (_Mdouble_ __x));`): the header
// parser expands them, so Linux finds the libm signatures too (fixture in the style of glibc)
use warp::ffi::{expand_glibc_math_macros, parse_header_file};

const GLIBC_STYLE_MATH_H: &str = "tests/fixtures/glibc_math/math.h";

#[test]
fn glibc_math_macros_expand_to_double_declarations() {
	assert_eq!(expand_glibc_math_macros("__MATHCALL (sqrt,, (_Mdouble_ __x));"), "double sqrt (double __x);");
	assert_eq!(expand_glibc_math_macros("__MATHDECL (int,ilogb,, (_Mdouble_ __x));"), "int ilogb (double __x);");
}

#[test]
fn a_glibc_style_math_header_declares_libm() {
	let signatures = parse_header_file(GLIBC_STYLE_MATH_H, "m");
	let names: Vec<&str> = signatures.iter().map(|signature| signature.name.as_str()).collect();
	for name in ["cos", "sqrt", "pow", "fabs", "ilogb"] {
		assert!(names.contains(&name), "{name} missing in {names:?}");
	}
	let pow = signatures.iter().find(|signature| signature.name == "pow").unwrap();
	assert_eq!(pow.param_types, vec!["double", "double"]);
}
