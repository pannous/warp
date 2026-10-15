// decision dimensions-units Q1, dimension-bridges: px is the device pixel, a dimension of its own, bridged to lengths by
// lib/units.warp's `length as pixels := it * display.dpi`, read at each conversion
use crate::common::fails_with;
use crate::is;

#[test]
fn pixels_are_their_own_dimension() {
	is!("3px + 2px == 5px", true);
	is!("type(3px)", "pixels");
	fails_with("3px + 1m", "DimensionError: px + m");
	fails_with("3cm + 2px", "as px");
}

// display.dpi is the screen's, 96 where the screen tells none (CI)
#[test]
fn pixels_turn_into_lengths_through_the_displays_dpi() {
	is!("x = 1 inch\n(x as px) == 1 inch * display.dpi", true);
	is!("w = 5px\n(w as mm) as px == 5px", true);
	is!("a = (10px)²\n(a as inch²) == (10px / display.dpi)²", true);
	is!("display.dpi = 300 dpi\n(1 inch as px) == 300px", true);
}
