//! card compound-unit: the whole unit after `as` is the target, as after `in`: `60 mi/h as km/h`, not `(60 mi/h as km)/h`
use warp::wasm_emitter::eval;

fn shown(code: &str) -> String {
	eval(code).serialize().trim().to_string()
}

#[test]
fn a_compound_unit_after_as_is_the_target() {
	assert_eq!(shown("60 mi/h as km/h"), "96.56064km/h");
	assert_eq!(shown("36 km/h as m/s"), "10m/s");
	assert_eq!(shown("1 kg*m/s² as g*cm/s²"), "100000g·cm/s²");
}

#[test]
fn a_run_time_quantity_converts_to_a_compound_unit() {
	assert_eq!(shown("x = 10 m\nt = 2 s\nx / t as km/h"), "18km/h");
	assert_eq!(shown("v = 60 mph\n\"v: \" + (v as km/h)"), "\"v: 96.56064km/h\"");
}
