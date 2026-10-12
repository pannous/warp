// card native-window: natively `on click {…}` hears the clicks over the paint window, event {x, y} in the painted
// frame's pixels, as the playground page raises click on its canvas. The clicks are told here as the viewer's input
// lines tell them (src/paint_window.rs follow_input): released, pressed at 30, 40, released; the key held stays
use warp_runtime::system_values::{tell_window_input, window_input};

#[test]
fn on_click_hears_the_paint_windows_clicks() {
	let key = window_input()[3];
	for [x, y, down] in [[0.0, 0.0, 0.0], [30.0, 40.0, 1.0], [30.0, 40.0, 0.0]] {
		tell_window_input([x, y, down, key]);
	}
	let heard = warp::wasm_emitter::eval(
		"clicks = \"\"\non click { clicks = clicks + event.x + \",\" + event.y + \" \" }\nsleep(100 ms)\nclicks",
	)
	.serialize();
	// another test may have pressed the button before: its click comes first
	assert!(heard.contains("30,40 "), "{heard}");
}
