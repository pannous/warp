// An angle in degrees: ° is π/180, so 30° is the exact π/6 and sin(30°) is 0.5 (card angles-degrees)
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_degree_is_pi_over_180() {
	assert_eq!(text_of("90°"), "π/2");
	assert_eq!(text_of("180° == π"), "yes");
	assert_eq!(text_of("angle = 45°; angle"), "π/4");
}

#[test]
fn trigonometry_takes_degrees() {
	assert_eq!(text_of("sin(30°)"), "0.5");
	assert_eq!(text_of("cos(180°)"), "-1");
	assert_eq!(text_of("x = 30; sin(x°)"), "0.5");
	assert_eq!(text_of("f(a) := sin(a); f(90°)"), "1");
}

#[test]
fn a_temperature_keeps_its_degree_sign() {
	assert_eq!(text_of("40°C as °F"), "104°F");
}
