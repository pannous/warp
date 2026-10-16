// card graphics-cpu-paint: the CPU painting of paint(f, width, height) is the prelude's warp word paint_each, which the
// lowering writes in place with f called by its name
#![cfg(feature = "native")]
use super::test_paint_lists::{painted_rows, painted_rows_by};

#[test]
fn paint_each_paints_a_function_pixel_by_pixel() {
	let mut on_cpu = crate::common::warp_command();
	on_cpu.env("WARP_GPU", "off");
	let each = painted_rows_by(on_cpu, "ink(x, y) := (x + y) % 2\npaint(ink, 2 + 2, 2)", "paint_each");
	let listed = painted_rows("paint([0, 1, 0, 1, 1, 0, 1, 0], 4, 2)", "paint_each_listed");
	assert_eq!(each, listed);
}
