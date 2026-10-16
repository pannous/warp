// card shader-click-ripples: samples/graphics/shader_brush.warp sends a ripple out from each click, its own hole $ripple (where
// the last click was, the seconds since), set by `on click`, natively from the paint window too. The click is told here
// as the viewer's input lines tell it (src/graphics/paint_window.rs follow_input); headless the sample paints one frame
use warp_runtime::system_values::{tell_window_input, window_input};

#[test]
fn a_click_starts_the_brushs_ripple_where_it_was() {
	let _window = crate::common::window_input_held();
	let key = window_input()[3];
	for [x, y, down] in [[0.0, 0.0, 0.0], [300.0, 200.0, 1.0], [300.0, 200.0, 0.0]] {
		tell_window_input([x, y, down, key]);
	}
	let sample =
		std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/samples/graphics/shader_brush.warp")).unwrap();
	let ripple =
		warp::wasm_emitter::eval(&format!("{sample}\nsleep(100 ms)\n\"\" + ripple#1 + \",\" + ripple#2")).serialize();
	// another test may have clicked before: the last click wins
	assert!(ripple.contains("300,200"), "{ripple}");
}
