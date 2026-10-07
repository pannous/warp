// `use draw` (std/draw.wasp, card drawing-words): a canvas of colored pixels, shapes and colors as 0xAARRGGBB numbers,
// shown by paint, which draws a value with an alpha byte in its color (src/paint.rs shade)
use crate::is;

const DRAW: &str = "use draw\n";

#[test]
fn colors_are_numbers_with_an_alpha_byte() {
	is!(&format!("{DRAW}rgb(255, 0, 0) == red"), true);
	is!(&format!("{DRAW}hsv(120, 1, 1) == rgb(0, 255, 0)"), true);
	is!(&format!("{DRAW}hsv(0, 1, 1) == red"), true);
	is!(&format!("{DRAW}alpha_of(with_alpha(blue, 0.5))"), 128);
}

#[test]
fn shapes_set_the_pixels_they_cover() {
	is!(&format!("{DRAW}canvas(3, 3)\ndot(1, 1, red)\ncanvas_pixels[4] == red"), true);
	is!(&format!("{DRAW}canvas(9, 9)\ncircle(4.5, 4.5, 2, blue)\n[canvas_pixels[4 * 9 + 4] == blue, canvas_pixels[0]]"), warp::ints(vec![1, 0]));
	is!(&format!("{DRAW}canvas(4, 4)\nrect(1, 1, 2, 2, green)\ncount(canvas_pixels.filter(p => p == green))"), 4);
	is!(&format!("{DRAW}canvas(5, 5)\nline(0, 0, 4, 4, black)\ncount(canvas_pixels.filter(p => p == black))"), 5);
}

#[test]
fn a_color_with_alpha_mixes_with_what_is_below() {
	is!(&format!("{DRAW}canvas(1, 1)\nclear(white)\ndot(0, 0, with_alpha(black, 0.5))\ncanvas_pixels[0] == rgb(127, 127, 127)"), true);
	is!(&format!("{DRAW}canvas(1, 1)\nclear(red)\ndot(0, 0, with_alpha(blue, 0))\ncanvas_pixels[0] == red"), true);
}
