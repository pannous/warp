// Temperatures are units with a zero of their own: °C, °F and K convert with an offset (card temperature)
use crate::common;
use warp::wasm_emitter::eval;

fn text_of(code: &str) -> String {
	eval(code).serialize()
}

#[test]
fn a_temperature_is_a_quantity() {
	assert_eq!(text_of("temperature = 40°C; temperature"), "40°C");
	assert_eq!(text_of("98.6°F"), "98.6°F");
	assert_eq!(text_of("300K"), "300K");
}

#[test]
fn temperatures_convert_with_their_zero() {
	assert_eq!(text_of("40°C as °F"), "104°F");
	assert_eq!(text_of("212°F as °C"), "100°C");
	assert_eq!(text_of("0°C as K"), "273.15K");
	assert_eq!(text_of("0K in °F"), "-459.67°F");
}

#[test]
fn temperatures_compare_at_their_zero() {
	assert_eq!(text_of("40°C > 100°F"), "yes");
	assert_eq!(text_of("0°C == 32°F"), "yes");
}

#[test]
fn temperatures_in_one_unit_add() {
	assert_eq!(text_of("20°C + 5°C"), "25°C");
	common::fails_with("20°C + 50°F", "one unit");
}
