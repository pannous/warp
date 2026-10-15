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
	fails_with("h: length = 3 kg", "DimensionError: h is declared m, is given kg");
	fails_with("w: mass = 3 m", "DimensionError: w is declared kg, is given m");
}

/// The draft's own example: a declared `height` is the program's, not the canvas's (was "a plain number cannot be
/// converted to m")
#[test]
fn a_declared_height_is_the_programs() {
	is!("height: length = 2 inch; height as cm", value("5.08 cm"));
	is!("width: cm = 3 cm; width + 1 cm", value("4 cm"));
}

#[test]
fn a_program_naming_a_dimension_keeps_its_own() {
	is!("length = 3; length + 1", 4);
}

fn value(code: &str) -> warp::Node {
	warp::wasm_emitter::eval(code)
}
