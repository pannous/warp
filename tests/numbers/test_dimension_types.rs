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

/// A unit after a list is each element's (user 2026-10-15, Q3 of dimensions-units); was "repeat or multiply?". A list of
/// quantities goes wherever a whole list does: assigned, shown, summed, nested
#[test]
fn a_unit_after_a_list_applies_to_each_element() {
	let three_and_four_cm = value("[3 cm, 4 cm]");
	assert_eq!(three_and_four_cm.serialize(), "[3cm 4cm]", "a list of quantities, not its text");
	is!("[3 4] cm", three_and_four_cm.clone());
	is!("xs = [3, 4]; ys = xs cm; ys", three_and_four_cm.clone());
	is!("f(v) := v; f([3, 4] cm)", three_and_four_cm.clone());
	is!("n = 3; [n, 4] cm", three_and_four_cm.clone());
	is!("x = 3 cm; [x, 4 cm]", three_and_four_cm);
	is!("if yes { [1 2] m }", value("[1 m, 2 m]"));
	is!("x = [3, 4] cm; x in m", value("[0.03 m, 0.04 m]"));
	is!("y = [3, 4] km/h; y", value("[3 km/h, 4 km/h]"));
	is!("[[1, 2], [3, 4]] m", value("[[1 m, 2 m], [3 m, 4 m]]"));
	is!("x = [3, 4] cm; str(x)", "[3cm 4cm]");
	fails_with("[1 m, 2 kg]", "DimensionError: a list of m and kg");
}

/// A list of quantities gives its elements: by index, in a loop, mapped, summed (dimensions-units Q3, reviewer)
#[test]
fn a_list_of_quantities_gives_quantities() {
	is!("([3, 4] cm)#2", value("4 cm"));
	is!("x = [3, 4] cm; x[1] + 1 cm", value("5 cm"));
	is!("x = [3 4] cm; x[0] as mm", value("30 mm"));
	is!("x = [3, 4] cm; x#1 + 1 cm", value("4 cm"));
	is!("total = 0 cm; for d in [3, 4] cm { total += d }; total", value("7 cm"));
	is!("x = [3, 4] cm; x.map(d => d * 2)", value("[6 cm, 8 cm]"));
	is!("x = [3, 4] m; x / 2 s", value("[1.5 m/s, 2 m/s]"));
	is!("sum([3, 4] cm)", value("7 cm"));
	is!("x = [3, 4] cm; max(x) > 3.5 cm", true);
	is!("x = [3, 4] cm; count x", 2);
	is!("x = [3, 4] cm; type(x)", "list of length");
	is!("x = [3, 4] cm; x + [5 cm]", value("[3 cm, 4 cm, 5 cm]"));
	fails_with("x = [3, 4] cm; x + 1 cm", "list of length + length: append [the quantity] or map the list");
}

/// Wherever a list goes, its quantities go: comprehensions, a call's list, a typed list, a matrix, a list grown from
/// empty (reviewer, dimensions-units Q3)
#[test]
fn every_list_value_carries_its_units() {
	is!("xs = [3, 4] cm; [x * 2 for x in xs]", value("[6 cm, 8 cm]"));
	is!("xs = [3, 4] cm; [x + 1 cm for x in xs]", value("[4 cm, 5 cm]"));
	is!("xs = [3, 4] cm; [x for x in xs]", value("[3 cm, 4 cm]"));
	is!("g() := [1, 2]; g() cm", value("[1 cm, 2 cm]"));
	is!("v: length[2] = [1, 2] m; v", value("[1 m, 2 m]"));
	fails_with("v: length[2] = [1, 2] kg", "DimensionError: v is declared list of length (m), is given list of mass");
	is!("xs = [[1, 2], [3, 4]] m; xs#2", value("[3 m, 4 m]"));
	is!("xs = []; xs += [3 cm]; xs", value("[3 cm]"));
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
