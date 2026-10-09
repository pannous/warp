// card paint-text: label(x, y, words, color, size) of `use draw` sets words in a real sans-serif font on the canvas paint
// shows: the host word text_coverage, natively a system font (src/text_raster.rs), the browser's canvas in the playground
use crate::is;

const DRAW: &str = "use draw\n";

#[test]
fn a_label_covers_pixels_in_its_color() {
	is!(&format!("{DRAW}canvas(60, 24)\nlabel(0, 0, \"Hi\", red, 20)\ncount(canvas_pixels.filter(p => p == red)) > 20"), true);
	is!(&format!("{DRAW}canvas(60, 24)\nlabel(0, 0, \"Hi\", red, 20)\ncount(canvas_pixels.filter(p => p != 0 and p != red)) > 0"), true);
}

#[test]
fn text_coverage_grows_with_the_size_and_the_words() {
	is!("text_coverage(\"Hi\", 32)[0] > text_coverage(\"Hi\", 16)[0]", true);
	is!("text_coverage(\"Hi there\", 16)[0] > text_coverage(\"Hi\", 16)[0]", true);
	is!("c = text_coverage(\"Hi\", 16)\ncount(c) == 2 + c[0] * c[1]", true);
}
