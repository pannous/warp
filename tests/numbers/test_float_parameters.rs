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

#[test]
fn test_float_local_inside_float_function() {
	is!("half(x:float) := {y=x/2; y+1}; half(1)", 1.5);
	is!("quarters(n) := {q=1/4; q*n}; quarters(2)", 0.5);
}

#[test] // samples/raytracer.wasp: `normalize(v) := mul(v, 1.0 / length(v))` with length returning a float typed s as Int
fn test_parameter_passed_a_float_returning_call() {
	is!("def half(x) := sqrt(x) / 2; def scale(v, s) := v * s; scale(2, 1.0 / half(16.0))", 1.0);
	is!("def f() := 0.5 * random() + 1.5; def g(s) := s * 2; g(f()) >= 3", true);
}
