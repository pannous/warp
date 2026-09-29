// `x:float` parameters are computed as f64 and float-returning functions return f64
use warp::wasm_emitter::eval;
use warp::*;

#[test]
fn test_float_parameter_keeps_fraction() {
	is!("half(x:float) := x/2; half(1.0)", 0.5);
	is!("half(x:float) := x/2; 1+half(1.0)", 1.5);
	is!("half(x:float) := x/2; half(3)", 1.5);
}

#[test]
fn test_float_parameter_keeps_all_digits() {
	is!("half(x:float) := x/2; half(180.17933438838418)*2 == 180.17933438838418", true);
}

#[test]
fn test_unknown_parameter_type_is_reported() {
	let result = eval("half(x:flaot) := x/2; half(1)");
	assert!(result.serialize().contains("unknown type flaot"), "{result:?}");
}

#[test]
fn test_user_function_shadows_ffi_name() {
	is!("pow(b, e) := e == 0 ? 1 : b * pow(b, e-1); pow(2, 10)", 1024);
}
