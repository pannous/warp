//! card call-name: a libm function only the C headers know (no entry in LIBM_F64_FUNCTIONS) links like the listed ones,
//! instead of compiling to its last argument (`x = exp2(3); x + 1` was 4)
use crate::is;

#[test]
fn a_libm_function_of_the_headers_links_by_itself() {
	crate::requires!(crate::common::MACOS_C_HEADERS);
	is!("exp2(3)", 8);
	is!("x = exp2(3); x + 1", 9);
	is!("cbrt(27) + erf(0)", 3);
}

#[test]
fn a_libm_function_with_an_int_parameter_says_so() {
	crate::requires!(crate::common::MACOS_C_HEADERS);
	crate::common::fails_with("ldexp(1, 3)", "ldexp is a libm function with a parameter other than a float");
}

/// glibc declares math.h through macros: the hand-linked table then serves every function the implicit import takes
#[test]
fn the_table_serves_every_listed_libm_function() {
	let engine = warp::util::gc_engine();
	let mut linker: wasmtime::Linker<warp::ffi::FfiState> = wasmtime::Linker::new(&engine);
	warp::ffi::link_libm(&mut linker, &engine, &[]).unwrap();
	let mut store = wasmtime::Store::new(&engine, warp::ffi::FfiState::new());
	let missing: Vec<&str> = warp::ffi::LIBM_F64_FUNCTIONS.iter().map(|(name, _)| *name).filter(|name| linker.get(&mut store, "m", name).is_err()).collect();
	assert!(missing.is_empty(), "not in LIBM_UNARY/LIBM_BINARY: {missing:?}");
}

/// card ffi-mixed: a C function with float and int parameters gets each in its register (ldexp gave its first argument)
#[test]
fn a_c_function_with_float_and_int_parameters() {
	crate::requires!(crate::common::MACOS_C_HEADERS);
	is!("import ldexp from \"m\"; ldexp(1.0, 3)", 8.0);
	is!("import ldexp from \"m\"; ldexp(3.0, -1)", 1.5);
}
