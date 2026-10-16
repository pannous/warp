//! card unit-shallow (decisions unit-types, unit-field-plain-number): a unit is a shallow number type of a field,
//! `display { density: dpi }`; same-quantity units convert, others are a DimensionError, a plain number takes the
//! field's unit with a fixable warning
use crate::common::fails_with;
use warp::wasm_emitter::eval;

const DISPLAY: &str = "class Display{density: dpi; diagonal: inch}";

fn shown(rest: &str) -> String {
	eval(&format!("{DISPLAY}\n{rest}")).serialize().trim().to_string()
}

#[test]
fn a_unit_field_takes_any_unit_of_its_quantity() {
	assert_eq!(shown("Display(118 px/cm, 24 inch).density == 118 px/cm"), "yes");
	assert_eq!(shown("Display(density: 96 dpi, diagonal: 61 cm).diagonal == 61 cm"), "yes");
	fails_with(&format!("{DISPLAY}\nDisplay(3 kg, 24 inch)"), "Display.density holds");
	fails_with(&format!("{DISPLAY}\nDisplay(density: 3 kg, diagonal: 24 inch)"), "Display.density holds");
}

#[test]
fn a_plain_number_takes_the_fields_unit_with_a_warning() {
	warp::errors::diagnostic::take_warnings();
	assert_eq!(shown("Display(96, 24).density == 96 dpi"), "yes");
	let messages: Vec<String> = warp::errors::diagnostic::take_warnings().iter().map(|w| w.to_string()).collect();
	assert!(
		messages.iter().any(|message| message.contains("Display.density holds dpi: write the unit, `96 dpi`")),
		"{messages:?}"
	);
	assert_eq!(shown("Display{density: 96, diagonal: 24 inch}.density == 96 dpi"), "yes");
}

// reviewer: any plain number, not only a written one, and a field assignment; messages name the unit as written
#[test]
fn a_plain_value_or_assignment_takes_the_fields_unit() {
	assert_eq!(shown("x = 96\nDisplay(x, 24 inch).density == 96 dpi"), "yes");
	assert_eq!(shown("Display(2*48, 24 inch).density == 96 dpi"), "yes");
	assert_eq!(shown("d = Display(96 dpi, 24 inch)\nd.density = 120\nd.density == 120 dpi"), "yes");
	fails_with(&format!("{DISPLAY}\nDisplay(3 kg, 24 inch)"), "Display.density holds px/m (dpi), is given kg");
}
