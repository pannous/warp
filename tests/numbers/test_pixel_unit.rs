// decision dimensions-units Q1: px is the device pixel, a dimension of its own; a length turns into px by plain
// arithmetic with the screen's density, `3 cm * display.dpi` (user 2026-10-16: no bridge machinery in the compiler)
use crate::common::fails_with;
use crate::is;

#[test]
fn pixels_are_their_own_dimension() {
	is!("3px + 2px == 5px", true);
	is!("type(3px)", "pixels");
	fails_with("3px + 1m", "DimensionError: px + m");
	fails_with("3cm + 2px", "incompatible units");
	fails_with("1 inch as px", "cannot be converted to px");
}

// display.dpi is the screen's, 96 where the screen tells none (CI)
#[test]
fn pixels_turn_into_lengths_through_the_displays_dpi() {
	is!("x = 1 inch * display.dpi\n(x / display.dpi) == 1 inch", true);
	is!("w = 5px\n(w / display.dpi) * display.dpi == 5px", true);
	is!("a = (10px)²\n(a / display.dpi²) as inch² == (10px / display.dpi)²", true);
	is!("display.dpi = 300 dpi\n(1 inch * display.dpi) == 300px", true);
}

// display-names (user 2026-10-16): ppi is dpi, a dot is a px; `unit ppi := dpi` names another word's expression
#[test]
fn ppi_and_dots_are_dpi_and_pixels() {
	is!("3ppi == 3dpi", true);
	is!("1inch * 96ppi == 96px", true);
	is!("2dot + 3px == 5px", true);
}

// reviewer: the refusal names the way, a unit word of lib/units.warp whose amount turns one quantity into the other
#[test]
fn a_refused_conversion_names_the_unit_to_multiply_by() {
	fails_with("3cm as px", "multiply by an amount in dpi");
	fails_with("3px as cm", "divide by an amount in dpi");
}
