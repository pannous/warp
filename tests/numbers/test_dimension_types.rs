//! Dimension names are types of quantities (card dimensions-units): `q is length`, a declared `h: length` checks the
//! dimension whatever the unit, and products and quotients of dimensions name derived ones (`length/duration`)
use crate::common::fails_with;
use crate::is;

#[test]
fn a_quantity_is_its_dimension() {
	is!("3 cm is length", true);
	is!("3 kg is length", false);
	is!("2 km/h is length", false);
	is!("4 m² is length²", true);
	is!("20°C is temperature", true);
	is!("x = 5 mi; x is length", true);
}

/// The time dimension is `duration`, a subtype of `time` (decision anchor-time, Q5 of dimensions-units)
#[test]
fn a_quantity_of_seconds_is_a_duration_and_a_time() {
	is!("3 s is duration", true);
	is!("3 s is time", true);
	is!("3 m is time", false);
	is!("speed: length/duration = 3 km/h; speed is length/duration", true);
	is!("x = 3 cm; now is time", true);
}

#[test]
fn a_declared_dimension_checks_any_unit_of_it() {
	is!("h: length = 3 cm; h + 1 cm", value("4 cm"));
	is!("h: length = 2 inch; h as cm", value("5.08 cm"));
	fails_with("h: length = 3 kg", "DimensionError: h is declared length (m), is given kg (mass)");
	fails_with("w: mass = 3 m", "DimensionError: w is declared mass (kg), is given m (length)");
	fails_with("speed: length/duration = 3 kg", "DimensionError: speed is declared m/s, is given kg (mass)");
}

/// The draft's own example: a declared `height` is the program's, not the canvas's (was "a plain number cannot be
/// converted to m")
#[test]
fn a_declared_height_is_the_programs() {
	is!("height: length = 2 inch; height as cm", value("5.08 cm"));
	is!("width: cm = 3 cm; width + 1 cm", value("4 cm"));
}

/// Derived dimensions with a name of their own (user 2026-10-15, Q4 of dimensions-units)
#[test]
fn speed_area_volume_and_frequency_are_named_dimensions() {
	is!("4 m² is area", true);
	is!("4 m is area", false);
	is!("2 m³ is volume", true);
	is!("2 km/h is speed", true);
	is!("2 Hz is frequency", true);
	is!("a: area = 3 m²; a + 1 m²", value("4 m²"));
	is!("s: speed = 3 km/h; s", value("3 km/h"));
	fails_with("a: area = 3 m", "DimensionError: a is declared area (m²), is given m (length)");
}

/// A declaration written as a unit shows as written (reviewer, dimensions-units)
#[test]
fn a_declared_unit_shows_as_written() {
	fails_with("h: cm = 3 kg", "DimensionError: h is declared cm, is given kg (mass)");
	fails_with("v: km/h = 3 kg", "DimensionError: v is declared km/h, is given kg (mass)");
}

#[test]
fn a_program_naming_a_dimension_keeps_its_own() {
	is!("length = 3; length + 1", 4);
}

fn value(code: &str) -> warp::Node {
	warp::wasm_emitter::eval(code)
}
