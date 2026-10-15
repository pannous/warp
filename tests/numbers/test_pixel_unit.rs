// decision dimensions-units Q1: px is the device pixel, a dimension of its own, turned into lengths through display.dpi
// at run time (an inch is display.dpi pixels)
use crate::common::fails_with;
use crate::is;

#[test]
fn pixels_are_their_own_dimension() {
	is!("3px + 2px == 5px", true);
	is!("type(3px)", "pixel");
	fails_with("3px + 1m", "DimensionError: px + m");
}

// the screen's dpi is the host's: a machine without a display (CI) has none to ask
#[cfg(target_os = "macos")]
#[test]
fn pixels_turn_into_lengths_through_the_displays_dpi() {
	is!("x = 1 inch\n(x as px) == display.dpi * 1px", true);
	is!("w = 5px\n(w as mm) as px == 5px", true);
	is!("a = (10px)²\n(a as inch²) == (10 / display.dpi)² * 1 inch²", true);
}
