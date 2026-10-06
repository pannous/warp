// paint(pixels, width, height) draws on the canvas of the browser playground (issue #15); natively there is no canvas
// yet, and saying so is loud, never a silent no-op
#![cfg(feature = "native")]
use crate::common::fails_with;

#[test]
fn paint_without_a_canvas_is_a_loud_error() {
	fails_with("paint([0, 1, 1, 0], 2, 2); 7", "paint draws on the canvas of the browser playground");
}
