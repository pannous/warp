// card make-width: window.width/height, display.width/height and canvas.width/height are properties, readable and
// writable (writing resizes; the display only reads); a bare width or height is the canvas's, with a hint naming the
// three. In tests no window opens: a window size written is the one read back (warp-runtime system_values.rs)
use crate::is;
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

#[test]
fn canvas_size_is_a_property() {
	is!("use draw\ncanvas(30, 20)\n[canvas.width, canvas.height]", warp::ints(vec![30, 20]));
	is!(
		"use draw\ncanvas.width = 300\ncanvas.height = 200\n[canvas.width, canvas.height, count(canvas_pixels)]",
		warp::ints(vec![300, 200, 60000])
	);
	is!("use draw\ncanvas(10, 10)\ncanvas.width += 5\ncanvas.width", 15);
	// draw comes along without `use draw`
	is!("canvas.width = 30\ncanvas.width", 30);
	is!("set width of canvas to 120\ncanvas.width", 120); // card english-setter
}

#[test]
fn a_bare_width_is_the_canvas_width() {
	is!("canvas(5, 6)\n[width, height]", warp::ints(vec![5, 6]));
	let (_, hints) = capture_hints(|| eval("canvas(5, 6)\nwidth"));
	let hint = hints.iter().find(|hint| hint.canonical == "canvas.width").expect("a did-you-mean hint");
	assert!(hint.reason.contains("window.width") && hint.reason.contains("display.width"), "{}", hint.reason);
}

#[test]
fn a_width_of_the_program_keeps_its_meaning() {
	is!("width = 3\nwidth", 3);
	is!("f(width) := width * 2\nf(4)", 8);
	is!("p = {width: 5}\np.width", 5);
	is!("canvas = {width: 9}\ncanvas.width", 9);
}

#[test]
fn window_size_is_a_property() {
	is!("window.width = 800\nwindow.height = 600\n[window.width, window.height]", warp::ints(vec![800, 600]));
	is!("window.height += 10\nwindow.height > 10", true);
}

#[test]
fn the_display_is_read_only() {
	let refused = format!("{:?}", eval("display.width = 5"));
	assert!(refused.contains("cannot resize the display"), "{refused}");
	#[cfg(target_os = "macos")]
	is!("display.width > 100 and display.height > 100", true);
}

// the viewer process takes `window.width = 800` as a message of its own: no window opens for it, a shown one resizes
#[cfg(feature = "native")]
#[test]
fn the_viewer_reads_a_resize() {
	use warp::graphics::paint_window::{read_message, resize_message, Message};
	let message = read_message(&mut resize_message(800, 600).as_slice());
	assert!(matches!(message, Some(Message::Resize(800, 600))));
}
